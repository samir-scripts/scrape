//! Extraction-field rules shared by workflows and saved-job repository queries.
use crate::enrich::salary::CompensationSqlValue as Bind;
use crate::enrich::{CompensationFilter, ParsedLocation};
use crate::{
    FilterField, JobFilterContext, RoleType, TriState, WorkAuthRequirement, WorkerEvent,
    WorkplaceType,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[ts(export)]
pub struct KeywordRules {
    pub any: Vec<String>,
    pub all: Vec<String>,
    pub exclude: Vec<String>,
    pub fields: Vec<FilterField>,
}
impl Default for KeywordRules {
    fn default() -> Self {
        Self {
            any: vec![],
            all: vec![],
            exclude: vec![],
            fields: vec![FilterField::Title],
        }
    }
}
impl KeywordRules {
    fn fields(&self) -> Vec<FilterField> {
        if self.fields.is_empty() {
            vec![
                FilterField::Title,
                FilterField::Description,
                FilterField::Company,
                FilterField::Location,
            ]
        } else {
            self.fields.clone()
        }
    }
    pub fn matches(&self, context: &JobFilterContext) -> bool {
        let contains = |word: &str| {
            self.fields().iter().any(|field| {
                context
                    .get_field_str(*field)
                    .to_ascii_lowercase()
                    .contains(&word.to_ascii_lowercase())
            })
        };
        (self.any.is_empty() || self.any.iter().any(|word| contains(word)))
            && self.all.iter().all(|word| contains(word))
            && !self.exclude.iter().any(|word| contains(word))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[ts(export)]
pub struct PostingRules {
    pub keywords: Option<KeywordRules>,
    pub posting_date: Option<crate::posting_date::PostingDateFilter>,
    pub compensation: Option<CompensationFilter>,
    pub role_types: Vec<RoleType>,
    pub workplaces: Vec<WorkplaceType>,
    #[ts(type = "number | null")]
    pub experience_min: Option<i64>,
    #[ts(type = "number | null")]
    pub experience_max: Option<i64>,
    pub experience_include_unknown: bool,
    /// "any", "only", or "exclude". Exclusion retains unknown classifications.
    pub new_grad: Option<String>,
    pub sponsorship: Vec<TriState>,
    pub work_auth: Vec<WorkAuthRequirement>,
    pub clearance: Vec<TriState>,
    pub countries: Vec<String>,
    pub regions: Vec<String>,
    pub location_include_unknown: bool,
}
impl PostingRules {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(keywords) = &self.keywords {
            if keywords
                .any
                .iter()
                .chain(&keywords.all)
                .chain(&keywords.exclude)
                .any(|word| word.trim().is_empty())
            {
                return Err("Keywords must not be blank".into());
            }
            if keywords.fields.contains(&FilterField::RoleType) {
                return Err(
                    "Keyword fields must be title, description, company, or location".into(),
                );
            }
        }
        if let Some(date) = &self.posting_date {
            date.bounds(chrono::Utc::now())?;
        }
        if let Some(pay) = &self.compensation {
            pay.validate()?;
        }
        if self.experience_min.is_some_and(|v| v < 0)
            || self.experience_max.is_some_and(|v| v < 0)
            || matches!((self.experience_min, self.experience_max), (Some(lo), Some(hi)) if lo > hi)
        {
            return Err("Invalid experience range".into());
        }
        if self
            .new_grad
            .as_deref()
            .is_some_and(|value| !matches!(value, "any" | "only" | "exclude"))
        {
            return Err("new_grad must be any, only, or exclude".into());
        }
        Ok(())
    }
    fn geography_matches(&self, locations: &[ParsedLocation]) -> bool {
        if self.countries.is_empty() && self.regions.is_empty() {
            return true;
        }
        if locations.is_empty() {
            return self.location_include_unknown;
        }
        locations.iter().any(|loc| {
            (self.countries.is_empty()
                || self.countries.iter().any(|country| {
                    loc.country_code
                        .as_ref()
                        .is_some_and(|code| code.eq_ignore_ascii_case(country))
                }))
                && (self.regions.is_empty()
                    || self.regions.iter().any(|region| {
                        loc.region_code.as_ref().is_some_and(|code| {
                            code.eq_ignore_ascii_case(region)
                                || code
                                    .to_ascii_uppercase()
                                    .ends_with(&format!("-{}", region.to_ascii_uppercase()))
                        })
                    }))
        })
    }
    pub fn matches_event(&self, event: &WorkerEvent) -> bool {
        self.matches_event_at(event, chrono::Utc::now())
    }
    pub fn matches_event_at(
        &self,
        event: &WorkerEvent,
        started: chrono::DateTime<chrono::Utc>,
    ) -> bool {
        if let Some(date) = &self.posting_date {
            if let WorkerEvent::Job { date_posted, .. } = event {
                if !date.matches(date_posted.as_deref(), started) {
                    return false;
                }
            }
        }
        let WorkerEvent::Job {
            title,
            company,
            location,
            description_text,
            role_type,
            workplace_type,
            salary,
            experience,
            is_new_grad,
            sponsorship,
            work_auth,
            clearance,
            locations,
            ..
        } = event
        else {
            return false;
        };
        let context = JobFilterContext {
            title,
            company,
            location,
            description: description_text.as_deref().unwrap_or(""),
            role_type: *role_type,
        };
        if self
            .keywords
            .as_ref()
            .is_some_and(|rules| !rules.matches(&context))
            || self.compensation.as_ref().is_some_and(|filter| {
                !filter.matches(salary.as_ref().map(|s| s.ranges.as_slice()).unwrap_or(&[]))
            })
        {
            return false;
        }
        if !self.role_types.is_empty() && !self.role_types.contains(role_type)
            || !self.workplaces.is_empty() && !self.workplaces.contains(workplace_type)
        {
            return false;
        }
        if self.experience_min.is_some() || self.experience_max.is_some() {
            let min = experience.as_ref().and_then(|e| e.min_years);
            let max = experience.as_ref().and_then(|e| e.max_years);
            if min.is_none() && max.is_none() {
                if !self.experience_include_unknown {
                    return false;
                }
            } else if self
                .experience_min
                .is_some_and(|lo| max.is_some_and(|hi| hi < lo))
                || self
                    .experience_max
                    .is_some_and(|hi| min.is_some_and(|lo| lo > hi))
            {
                return false;
            }
        }
        if self.new_grad.as_deref() == Some("only") && *is_new_grad != TriState::Yes
            || self.new_grad.as_deref() == Some("exclude") && *is_new_grad == TriState::Yes
        {
            return false;
        }
        if !self.sponsorship.is_empty()
            && !self
                .sponsorship
                .contains(&sponsorship.as_ref().map(|s| s.value).unwrap_or_default())
            || !self.work_auth.is_empty()
                && !self
                    .work_auth
                    .contains(&work_auth.as_ref().map(|s| s.value).unwrap_or_default())
            || !self.clearance.is_empty()
                && !self
                    .clearance
                    .contains(&clearance.as_ref().map(|s| s.value).unwrap_or_default())
        {
            return false;
        }
        self.geography_matches(locations)
    }
    pub fn sqlite_predicates(&self) -> (Vec<String>, Vec<Bind>) {
        self.sqlite_predicates_at(chrono::Utc::now())
    }
    pub fn sqlite_predicates_at(
        &self,
        started: chrono::DateTime<chrono::Utc>,
    ) -> (Vec<String>, Vec<Bind>) {
        let mut clauses = Vec::new();
        let mut bindings = Vec::new();
        fn choices<T: Serialize>(
            column: &str,
            values: &[T],
            clauses: &mut Vec<String>,
            bindings: &mut Vec<Bind>,
        ) {
            if values.is_empty() {
                return;
            }
            clauses.push(format!(
                "{column} IN ({})",
                vec!["?"; values.len()].join(",")
            ));
            bindings.extend(
                values
                    .iter()
                    .map(|v| Bind::Text(serde_json::to_value(v).unwrap().as_str().unwrap().into())),
            );
        }
        if let Some(date) = &self.posting_date {
            if let Ok((start, end)) = date.bounds(started) {
                let instant = "COALESCE((SELECT instant FROM posting_date_cache WHERE date_text = jobs.date_posted AND timezone = ?), jobs.date_posted)";
                let mut date_clauses = Vec::new();
                if let Some(start) = start {
                    date_clauses.push(format!("julianday({instant}) >= julianday(?)"));
                    bindings.push(Bind::Text(date.timezone.clone()));
                    bindings.push(Bind::Text(start.to_rfc3339()));
                }
                if let Some(end) = end {
                    date_clauses.push(format!("julianday({instant}) < julianday(?)"));
                    bindings.push(Bind::Text(date.timezone.clone()));
                    bindings.push(Bind::Text(end.to_rfc3339()));
                }
                let known =
                    "EXISTS (SELECT 1 FROM posting_date_cache WHERE date_text = jobs.date_posted)";
                let range = if date_clauses.is_empty() {
                    known.to_string()
                } else {
                    date_clauses.join(" AND ")
                };
                clauses.push(if date.include_unknown {
                    format!("(NOT ({known}) OR ({range}))")
                } else {
                    format!("({known} AND ({range}))")
                });
            } else {
                clauses.push("0".into());
            }
        }
        if let Some(keywords) = &self.keywords {
            for (words, join, negate) in [
                (&keywords.any, " OR ", false),
                (&keywords.all, " AND ", false),
                (&keywords.exclude, " OR ", true),
            ] {
                if words.is_empty() {
                    continue;
                }
                let mut word_clauses = Vec::new();
                for word in words {
                    let mut fields = Vec::new();
                    for field in keywords.fields() {
                        let column = match field {
                            FilterField::Title => "title",
                            FilterField::Description => "description_text",
                            FilterField::Company => "company",
                            FilterField::Location => "location",
                            FilterField::RoleType => "role_type",
                        };
                        fields.push(format!(
                            "instr(lower(COALESCE({column}, '')), lower(?)) > 0"
                        ));
                        bindings.push(Bind::Text(word.clone()));
                    }
                    word_clauses.push(format!("({})", fields.join(" OR ")));
                }
                clauses.push(format!(
                    "{}({})",
                    if negate { "NOT " } else { "" },
                    word_clauses.join(join)
                ));
            }
        }
        if let Some(pay) = &self.compensation {
            let (sql, values) = pay.sqlite_predicate();
            clauses.push(sql);
            bindings.extend(values);
        }
        choices("role_type", &self.role_types, &mut clauses, &mut bindings);
        choices(
            "workplace_type",
            &self.workplaces,
            &mut clauses,
            &mut bindings,
        );
        choices(
            "sponsorship",
            &self.sponsorship,
            &mut clauses,
            &mut bindings,
        );
        choices("work_auth", &self.work_auth, &mut clauses, &mut bindings);
        choices("clearance", &self.clearance, &mut clauses, &mut bindings);
        if self.experience_min.is_some() || self.experience_max.is_some() {
            let mut overlap = Vec::new();
            if let Some(lo) = self.experience_min {
                overlap.push("(experience_max IS NULL OR experience_max >= ?)");
                bindings.push(Bind::Integer(lo));
            }
            if let Some(hi) = self.experience_max {
                overlap.push("(experience_min IS NULL OR experience_min <= ?)");
                bindings.push(Bind::Integer(hi));
            }
            let known = "(experience_min IS NOT NULL OR experience_max IS NOT NULL)";
            let overlap = overlap.join(" AND ");
            clauses.push(if self.experience_include_unknown {
                format!("(NOT {known} OR ({overlap}))")
            } else {
                format!("({known} AND ({overlap}))")
            });
        }
        match self.new_grad.as_deref() {
            Some("only") => clauses.push("is_new_grad = 'yes'".into()),
            Some("exclude") => clauses.push("is_new_grad != 'yes'".into()),
            _ => {}
        }
        if !self.countries.is_empty() || !self.regions.is_empty() {
            let mut geo = Vec::new();
            if !self.countries.is_empty() {
                geo.push(format!(
                    "upper(jl.country_code) IN ({})",
                    vec!["?"; self.countries.len()].join(",")
                ));
                bindings.extend(
                    self.countries
                        .iter()
                        .map(|c| Bind::Text(c.to_ascii_uppercase())),
                );
            }
            if !self.regions.is_empty() {
                let mut parts = Vec::new();
                for region in &self.regions {
                    parts.push("(upper(jl.region_code) = ? OR substr(upper(jl.region_code), instr(jl.region_code, '-') + 1) = ?)");
                    bindings.push(Bind::Text(region.to_ascii_uppercase()));
                    bindings.push(Bind::Text(region.to_ascii_uppercase()));
                }
                geo.push(format!("({})", parts.join(" OR ")));
            }
            let exists = format!(
                "EXISTS (SELECT 1 FROM job_locations jl WHERE jl.job_id=jobs.id AND ({}))",
                geo.join(" AND ")
            );
            clauses.push(if self.location_include_unknown { format!("({exists} OR NOT EXISTS (SELECT 1 FROM job_locations jl WHERE jl.job_id=jobs.id))") } else { exists });
        }
        (clauses, bindings)
    }
}
