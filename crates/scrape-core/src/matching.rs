//! Extraction-field rules shared by workflows and saved-job repository queries.
use crate::enrich::salary::CompensationSqlValue as Bind;
use crate::enrich::{CompensationFilter, ParsedLocation};
use crate::{
    FilterField, JobFilterContext, RoleType, TriState, WorkAuthRequirement, WorkerEvent,
    WorkplaceType,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Stable primary categories. Detailed failures may overlap; primary counts do not.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RejectionReason {
    Query,
    Policy,
    LegacyKeywords,
    LegacyExclusion,
    LegacyLocation,
    LegacyRoleType,
    KeywordAny,
    KeywordAll,
    KeywordExclusion,
    PostingDate,
    Compensation,
    RoleType,
    Workplace,
    Experience,
    NewGrad,
    Sponsorship,
    WorkAuthorization,
    Clearance,
    Geography,
    InvalidCandidate,
}

/// Compiles legacy expressions once and uses the same predicates for decisions and diagnostics.
pub struct MatchingEvaluator<'a> {
    filters: &'a crate::WorkerFilters,
    predicates: Vec<(RejectionReason, crate::FilterExpr)>,
}
impl<'a> MatchingEvaluator<'a> {
    pub fn new(filters: &'a crate::WorkerFilters) -> Result<Self, String> {
        Ok(Self {
            filters,
            predicates: filters.categorized_predicates()?,
        })
    }
    pub fn rejection_reasons(
        &self,
        event: &WorkerEvent,
        started: chrono::DateTime<chrono::Utc>,
    ) -> Vec<RejectionReason> {
        let WorkerEvent::Job {
            title,
            location,
            company,
            description_text,
            role_type,
            salary,
            ..
        } = event
        else {
            return vec![RejectionReason::InvalidCandidate];
        };
        let context = JobFilterContext {
            title,
            location,
            company,
            description: description_text.as_deref().unwrap_or(""),
            role_type: *role_type,
        };
        let mut reasons: Vec<_> = self
            .predicates
            .iter()
            .filter(|(_, predicate)| !predicate.eval(&context))
            .map(|(reason, _)| *reason)
            .collect();
        if let Some(rules) = &self.filters.matching {
            reasons.extend(rules.rejection_reasons_at(event, started));
        }
        if self.filters.compensation.as_ref().is_some_and(|pay| {
            !pay.matches(salary.as_ref().map(|s| s.ranges.as_slice()).unwrap_or(&[]))
        }) {
            reasons.push(RejectionReason::Compensation);
        }
        // Legacy and structured compensation can fail together, but describe one category.
        let mut seen = std::collections::BTreeSet::new();
        reasons.retain(|reason| seen.insert(*reason));
        reasons
    }
    pub fn matches_at(&self, event: &WorkerEvent, started: chrono::DateTime<chrono::Utc>) -> bool {
        self.rejection_reasons(event, started).is_empty()
    }
}

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
        self.rejection_reasons(context).is_empty()
    }
    pub fn rejection_reasons(&self, context: &JobFilterContext) -> Vec<RejectionReason> {
        let contains = |word: &str| {
            self.fields().iter().any(|field| {
                context
                    .get_field_str(*field)
                    .to_ascii_lowercase()
                    .contains(&word.to_ascii_lowercase())
            })
        };
        let mut reasons = Vec::new();
        if !self.any.is_empty() && !self.any.iter().any(|word| contains(word)) {
            reasons.push(RejectionReason::KeywordAny);
        }
        if !self.all.iter().all(|word| contains(word)) {
            reasons.push(RejectionReason::KeywordAll);
        }
        if self.exclude.iter().any(|word| contains(word)) {
            reasons.push(RejectionReason::KeywordExclusion);
        }
        reasons
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
        if self.countries.iter().any(|value| {
            !crate::geography::country_code(value)
                .is_some_and(|code| code.eq_ignore_ascii_case(value))
        }) {
            return Err(
                "matching.countries: expected ISO country codes such as US, CA or GB".into(),
            );
        }
        if self.regions.iter().any(|value| {
            let short = value
                .split_once('-')
                .map_or(value.as_str(), |(_, suffix)| suffix);
            let prefix_valid = value.split_once('-').is_none_or(|(prefix, _)| {
                crate::geography::country_code(prefix)
                    .is_some_and(|code| code.eq_ignore_ascii_case(prefix))
            });
            !prefix_valid
                || short.is_empty()
                || short.len() > 3
                || !short.chars().all(|ch| ch.is_ascii_alphanumeric())
        }) {
            return Err("matching.regions: expected subdivision codes such as US-CA, CA-ON, or legacy short codes".into());
        }
        if !self.countries.is_empty()
            && !self.regions.is_empty()
            && !self.regions.iter().any(|region| {
                region.split_once('-').is_none_or(|(prefix, _)| {
                    self.countries
                        .iter()
                        .any(|country| country.eq_ignore_ascii_case(prefix))
                })
            })
        {
            return Err(
                "matching.regions: subdivision countries do not overlap the selected countries"
                    .into(),
            );
        }
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
        let known: Vec<_> = locations
            .iter()
            .filter(|loc| {
                loc.effective_country().is_some()
                    || loc
                        .region_code
                        .as_ref()
                        .is_some_and(|v| !v.trim().is_empty())
            })
            .collect();
        if known.is_empty() {
            return self.location_include_unknown;
        }
        known.into_iter().any(|loc| {
            let country = loc.effective_country();
            (self.countries.is_empty()
                || country.map_or(self.location_include_unknown, |code| {
                    self.countries.iter().any(|c| c.eq_ignore_ascii_case(code))
                }))
                && (self.regions.is_empty()
                    || if let Some(code) =
                        loc.region_code.as_deref().filter(|v| !v.trim().is_empty())
                    {
                        self.regions.iter().any(|region| {
                            code.eq_ignore_ascii_case(region)
                                || code
                                    .to_ascii_uppercase()
                                    .ends_with(&format!("-{}", region.to_ascii_uppercase()))
                        })
                    } else {
                        self.location_include_unknown
                            && self.regions.iter().any(|region| {
                                region.split_once('-').is_none_or(|(prefix, _)| {
                                    country.is_none_or(|code| code.eq_ignore_ascii_case(prefix))
                                })
                            })
                    })
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
        self.rejection_reasons_at(event, started).is_empty()
    }
    pub fn rejection_reasons_at(
        &self,
        event: &WorkerEvent,
        started: chrono::DateTime<chrono::Utc>,
    ) -> Vec<RejectionReason> {
        let mut reasons = Vec::new();
        let WorkerEvent::Job {
            date_posted,
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
            return vec![RejectionReason::InvalidCandidate];
        };
        let context = JobFilterContext {
            title,
            company,
            location,
            description: description_text.as_deref().unwrap_or(""),
            role_type: *role_type,
        };
        if let Some(date) = &self.posting_date {
            if !date.matches(date_posted.as_deref(), started) {
                reasons.push(RejectionReason::PostingDate);
            }
        }
        if let Some(keywords) = &self.keywords {
            reasons.extend(keywords.rejection_reasons(&context));
        }
        if self.compensation.as_ref().is_some_and(|filter| {
            !filter.matches(salary.as_ref().map(|s| s.ranges.as_slice()).unwrap_or(&[]))
        }) {
            reasons.push(RejectionReason::Compensation);
        }
        if !self.role_types.is_empty() && !self.role_types.contains(role_type) {
            reasons.push(RejectionReason::RoleType);
        }
        if !self.workplaces.is_empty() && !self.workplaces.contains(workplace_type) {
            reasons.push(RejectionReason::Workplace);
        }
        if self.experience_min.is_some() || self.experience_max.is_some() {
            let min = experience.as_ref().and_then(|e| e.min_years);
            let max = experience.as_ref().and_then(|e| e.max_years);
            if min.is_none() && max.is_none() {
                if !self.experience_include_unknown {
                    reasons.push(RejectionReason::Experience);
                }
            } else if self
                .experience_min
                .is_some_and(|lo| max.is_some_and(|hi| hi < lo))
                || self
                    .experience_max
                    .is_some_and(|hi| min.is_some_and(|lo| lo > hi))
            {
                reasons.push(RejectionReason::Experience);
            }
        }
        if self.new_grad.as_deref() == Some("only") && *is_new_grad != TriState::Yes
            || self.new_grad.as_deref() == Some("exclude") && *is_new_grad == TriState::Yes
        {
            reasons.push(RejectionReason::NewGrad);
        }
        if !self.sponsorship.is_empty()
            && !self
                .sponsorship
                .contains(&sponsorship.as_ref().map(|s| s.value).unwrap_or_default())
        {
            reasons.push(RejectionReason::Sponsorship);
        }
        if !self.work_auth.is_empty()
            && !self
                .work_auth
                .contains(&work_auth.as_ref().map(|s| s.value).unwrap_or_default())
        {
            reasons.push(RejectionReason::WorkAuthorization);
        }
        if !self.clearance.is_empty()
            && !self
                .clearance
                .contains(&clearance.as_ref().map(|s| s.value).unwrap_or_default())
        {
            reasons.push(RejectionReason::Clearance);
        }
        if !self.geography_matches(locations) {
            reasons.push(RejectionReason::Geography);
        }
        reasons
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
            let country = "coalesce(NULLIF(trim(jl.country_code),''), CASE WHEN instr(jl.region_code,'-') > 0 THEN substr(jl.region_code,1,instr(jl.region_code,'-')-1) END)";
            if !self.countries.is_empty() {
                let condition = format!(
                    "upper({country}) IN ({})",
                    vec!["?"; self.countries.len()].join(",")
                );
                geo.push(if self.location_include_unknown {
                    format!("({condition} OR {country} IS NULL)")
                } else {
                    condition
                });
                bindings.extend(
                    self.countries
                        .iter()
                        .map(|c| Bind::Text(c.to_ascii_uppercase())),
                );
            }
            if !self.regions.is_empty() {
                let mut parts = Vec::new();
                for region in &self.regions {
                    let condition = "(upper(jl.region_code) = ? OR substr(upper(jl.region_code), instr(jl.region_code, '-') + 1) = ?)";
                    bindings.push(Bind::Text(region.to_ascii_uppercase()));
                    bindings.push(Bind::Text(region.to_ascii_uppercase()));
                    parts.push(if self.location_include_unknown {
                        if let Some((prefix,_)) = region.split_once('-') {
                            bindings.push(Bind::Text(prefix.to_ascii_uppercase()));
                            format!("({condition} OR (NULLIF(trim(jl.region_code),'') IS NULL AND ({country} IS NULL OR upper({country}) = ?)))")
                        } else { format!("({condition} OR NULLIF(trim(jl.region_code),'') IS NULL)") }
                    } else { condition.to_string() });
                }
                geo.push(format!("({})", parts.join(" OR ")));
            }
            let known = "(NULLIF(trim(jl.country_code),'') IS NOT NULL OR NULLIF(trim(jl.region_code),'') IS NOT NULL)";
            let exists = format!("EXISTS (SELECT 1 FROM job_locations jl WHERE jl.job_id=jobs.id AND {known} AND ({}))", geo.join(" AND "));
            clauses.push(if self.location_include_unknown { format!("({exists} OR NOT EXISTS (SELECT 1 FROM job_locations jl WHERE jl.job_id=jobs.id AND {known}))") } else { exists });
        }
        (clauses, bindings)
    }
}

#[cfg(test)]
mod geography_tests {
    use super::*;
    fn location(country: Option<&str>, region: Option<&str>) -> ParsedLocation {
        ParsedLocation {
            raw: "fixture".into(),
            city: None,
            country_code: country.map(str::to_string),
            region_code: region.map(str::to_string),
            is_remote: true,
        }
    }
    #[test]
    fn unknown_geography_and_qualified_subdivisions_do_not_broaden_known_mismatches() {
        let mut rules = PostingRules {
            countries: vec!["US".into()],
            regions: vec!["US-CA".into()],
            ..Default::default()
        };
        assert!(!rules.geography_matches(&[location(None, None)]));
        rules.location_include_unknown = true;
        assert!(rules.geography_matches(&[location(None, None)]));
        assert!(rules.geography_matches(&[location(Some("US"), None)]));
        assert!(!rules.geography_matches(&[location(None, None), location(Some("CA"), None)]));
        assert!(!rules.geography_matches(&[location(None, Some("CA-ON"))]));
        rules.countries.push("CA".into());
        rules.regions = vec!["CA-ON".into()];
        assert!(!rules.geography_matches(&[location(Some("US"), None)]));
        assert!(rules.geography_matches(&[location(None, Some("CA-ON"))]));
    }
    #[test]
    fn malformed_or_impossible_geography_fails_validation() {
        for rules in [
            PostingRules {
                countries: vec!["ZZ".into()],
                ..Default::default()
            },
            PostingRules {
                regions: vec!["US/CA".into()],
                ..Default::default()
            },
            PostingRules {
                countries: vec!["US".into()],
                regions: vec!["CA-ON".into()],
                ..Default::default()
            },
        ] {
            assert!(rules.validate().is_err());
        }
    }
}

#[cfg(test)]
mod diagnostics_tests {
    use super::*;
    #[test]
    fn decisions_and_overlapping_reasons_share_the_same_predicates() {
        let filters: crate::WorkerFilters = serde_json::from_value(serde_json::json!({
            "query":"title LIKE '%Engineer%'", "policy":{"title":"%Rust%"},
            "include_keywords":["Developer"], "exclude_keywords":["Senior"],
            "locations":["Austin"], "role_types":["internship"],
            "matching":{"keywords":{"any":["Engineer"],"all":["Rust"],"exclude":["Senior"],"fields":["title"]},"countries":["US"],"role_types":["internship"]}
        })).unwrap();
        let evaluator = MatchingEvaluator::new(&filters).unwrap();
        let started = chrono::DateTime::parse_from_rfc3339("2026-10-16T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let event: WorkerEvent = serde_json::from_value(serde_json::json!({"type":"job","source_id":"fixture","external_id":"1","title":"Senior Engineer","company":"Fixture","url":"https://fixture.test/1","location":"Remote","role_type":"full_time"})).unwrap();
        let reasons = evaluator.rejection_reasons(&event, started);
        assert!(!evaluator.matches_at(&event, started));
        assert!(reasons.contains(&RejectionReason::Policy));
        assert!(reasons.contains(&RejectionReason::LegacyKeywords));
        assert!(reasons.contains(&RejectionReason::LegacyExclusion));
        assert!(reasons.contains(&RejectionReason::LegacyLocation));
        assert!(reasons.contains(&RejectionReason::LegacyRoleType));
        assert!(reasons.contains(&RejectionReason::KeywordAll));
        assert!(reasons.contains(&RejectionReason::KeywordExclusion));
        assert!(reasons.contains(&RejectionReason::RoleType));
        assert!(reasons.contains(&RejectionReason::Geography));
        assert!(!reasons.contains(&RejectionReason::KeywordAny));
        let context = JobFilterContext {
            title: "Senior Engineer",
            location: "Remote",
            description: "",
            company: "Fixture",
            role_type: RoleType::FullTime,
        };
        assert_eq!(
            filters.to_expr().unwrap().eval(&context),
            !reasons.iter().any(|r| matches!(
                r,
                RejectionReason::Query
                    | RejectionReason::Policy
                    | RejectionReason::LegacyKeywords
                    | RejectionReason::LegacyExclusion
                    | RejectionReason::LegacyLocation
                    | RejectionReason::LegacyRoleType
            ))
        );
        assert_eq!(
            filters
                .matching
                .as_ref()
                .unwrap()
                .matches_event_at(&event, started),
            filters
                .matching
                .as_ref()
                .unwrap()
                .rejection_reasons_at(&event, started)
                .is_empty()
        );
    }
}
