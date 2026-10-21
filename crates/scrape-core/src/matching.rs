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

/// Adapter-declared immutable evidence. Missing fields may still be supplied by details.
#[derive(Debug, Clone, Default)]
pub struct MatchingEvidence {
    pub fields: Vec<FilterField>,
    pub posting_date: bool,
    pub compensation: bool,
    /// Individually resolved structured fields, independent of description presence.
    pub known: Vec<RejectionReason>,
    /// A failed/exhausted opportunity can use explicitly permitted unknown values.
    pub opportunity_exhausted: bool,
}
impl MatchingEvidence {
    pub fn complete() -> Self {
        Self {
            fields: vec![
                FilterField::Title,
                FilterField::Location,
                FilterField::Description,
                FilterField::Company,
                FilterField::RoleType,
            ],
            posting_date: true,
            compensation: true,
            known: vec![RejectionReason::Workplace, RejectionReason::Geography],
            opportunity_exhausted: true,
        }
    }
    pub fn from_event(event: &WorkerEvent, opportunity_exhausted: bool) -> Self {
        let WorkerEvent::Job {
            location,
            description_text,
            date_posted,
            salary,
            workplace_type,
            location_data_complete,
            sponsorship,
            work_auth,
            clearance,
            experience,
            is_new_grad,
            ..
        } = event
        else {
            return Self::default();
        };
        let mut fields = vec![
            FilterField::Title,
            FilterField::Company,
            FilterField::RoleType,
        ];
        if !location.trim().is_empty() {
            fields.push(FilterField::Location);
        }
        if description_text
            .as_ref()
            .is_some_and(|d| !d.trim().is_empty())
        {
            fields.push(FilterField::Description);
        }
        let mut known = Vec::new();
        if *workplace_type != WorkplaceType::Unknown {
            known.push(RejectionReason::Workplace);
        }
        if *location_data_complete {
            known.push(RejectionReason::Geography);
        }
        if sponsorship
            .as_ref()
            .is_some_and(|value| value.value != TriState::Unknown)
        {
            known.push(RejectionReason::Sponsorship);
        }
        if work_auth
            .as_ref()
            .is_some_and(|value| value.value != WorkAuthRequirement::Unknown)
        {
            known.push(RejectionReason::WorkAuthorization);
        }
        if clearance
            .as_ref()
            .is_some_and(|value| value.value != TriState::Unknown)
        {
            known.push(RejectionReason::Clearance);
        }
        if experience
            .as_ref()
            .is_some_and(|value| value.min_years.is_some() || value.max_years.is_some())
        {
            known.push(RejectionReason::Experience);
        }
        if *is_new_grad != TriState::Unknown {
            known.push(RejectionReason::NewGrad);
        }
        Self {
            fields,
            known,
            posting_date: date_posted.is_some(),
            compensation: salary.is_some(),
            opportunity_exhausted,
        }
    }
    fn knows(&self, reason: RejectionReason) -> bool {
        if self.known.contains(&reason) {
            return true;
        }
        let has = |field| self.fields.contains(&field);
        match reason {
            RejectionReason::PostingDate => self.posting_date,
            RejectionReason::Compensation => self.compensation,
            RejectionReason::RoleType => has(FilterField::RoleType),
            RejectionReason::Geography | RejectionReason::Workplace => false,
            RejectionReason::Experience | RejectionReason::NewGrad => {
                has(FilterField::Description) && has(FilterField::Title)
            }
            RejectionReason::Sponsorship
            | RejectionReason::WorkAuthorization
            | RejectionReason::Clearance => has(FilterField::Description),
            _ => false,
        }
    }
}
#[derive(Debug, Default, PartialEq)]
pub struct MatchingAssessment {
    pub rejected: Vec<RejectionReason>,
    pub unresolved: Vec<RejectionReason>,
}
impl MatchingAssessment {
    fn check(&mut self, reason: RejectionReason, decision: Option<bool>) {
        let target = match decision {
            Some(true) => return,
            Some(false) => &mut self.rejected,
            None => &mut self.unresolved,
        };
        if !target.contains(&reason) {
            target.push(reason);
        }
    }
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
    pub fn assess(
        &self,
        event: &WorkerEvent,
        started: chrono::DateTime<chrono::Utc>,
        evidence: &MatchingEvidence,
    ) -> MatchingAssessment {
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
            return MatchingAssessment {
                rejected: vec![RejectionReason::InvalidCandidate],
                unresolved: vec![],
            };
        };
        let context = JobFilterContext {
            title,
            location,
            company,
            description: description_text.as_deref().unwrap_or(""),
            role_type: *role_type,
        };
        let mut assessment = MatchingAssessment::default();
        for (reason, predicate) in &self.predicates {
            assessment.check(*reason, predicate.eval_known(&context, &evidence.fields));
        }
        if let Some(rules) = &self.filters.matching {
            let structured = rules.assess(event, started, evidence);
            for reason in structured.rejected {
                assessment.check(reason, Some(false));
            }
            for reason in structured.unresolved {
                assessment.check(reason, None);
            }
        }
        if let Some(pay) = &self.filters.compensation {
            let matches = pay.matches(salary.as_ref().map(|s| s.ranges.as_slice()).unwrap_or(&[]));
            let known = evidence.compensation
                || (evidence.opportunity_exhausted && pay.include_unknown && salary.is_none());
            assessment.check(RejectionReason::Compensation, known.then_some(matches));
        }
        assessment
            .unresolved
            .retain(|reason| !assessment.rejected.contains(reason));
        assessment
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
        self.decisions(context, &MatchingEvidence::complete().fields)
            .into_iter()
            .filter_map(|(reason, matches)| (matches == Some(false)).then_some(reason))
            .collect()
    }
    fn decisions(
        &self,
        context: &JobFilterContext,
        known: &[FilterField],
    ) -> Vec<(RejectionReason, Option<bool>)> {
        use crate::filter::{known_all, known_any};
        let fields = self.fields();
        let contains = |word: &str| {
            known_any(fields.iter().map(|field| {
                known.contains(field).then(|| {
                    context
                        .get_field_str(*field)
                        .to_ascii_lowercase()
                        .contains(&word.to_ascii_lowercase())
                })
            }))
        };
        let mut decisions = Vec::new();
        if !self.any.is_empty() {
            decisions.push((
                RejectionReason::KeywordAny,
                known_any(self.any.iter().map(|word| contains(word))),
            ));
        }
        if !self.all.is_empty() {
            decisions.push((
                RejectionReason::KeywordAll,
                known_all(self.all.iter().map(|word| contains(word))),
            ));
        }
        if !self.exclude.is_empty() {
            decisions.push((
                RejectionReason::KeywordExclusion,
                known_any(self.exclude.iter().map(|word| contains(word))).map(|value| !value),
            ));
        }
        decisions
    }
}

/// Numeric experience rules retain overlap semantics unless explicitly converted.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ExperienceMatchMode {
    #[default]
    AdvertisedRangeOverlap,
    RequiredMinimum,
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
    pub experience_match_mode: ExperienceMatchMode,
    /// "any", "only", or "exclude". Exclusion retains unknown classifications.
    pub new_grad: Option<String>,
    pub sponsorship: Vec<TriState>,
    pub work_auth: Vec<WorkAuthRequirement>,
    pub clearance: Vec<TriState>,
    pub countries: Vec<String>,
    pub regions: Vec<String>,
    pub cities: Vec<String>,
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
        if self.cities.iter().any(|c| c.trim().is_empty()) {
            return Err("matching.cities: must not contain blank entries".into());
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
        if self.countries.is_empty() && self.regions.is_empty() && self.cities.is_empty() {
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
                    || loc.city.as_ref().is_some_and(|v| !v.trim().is_empty())
            })
            .collect();
        // Raw-only tuples can establish a city substring, but must never let a
        // Remote/unknown row bypass contradictory structured geography elsewhere.
        let candidates = if known.is_empty() {
            locations.iter().collect()
        } else {
            known
        };
        if candidates.is_empty() {
            return self.location_include_unknown;
        }
        candidates.into_iter().any(|loc| {
            let country = loc.effective_country();
            let country_match = self.countries.is_empty()
                || country.map_or(self.location_include_unknown, |code| {
                    self.countries.iter().any(|c| c.eq_ignore_ascii_case(code))
                });
            let region_match = self.regions.is_empty()
                || if let Some(code) = loc.region_code.as_deref().filter(|v| !v.trim().is_empty()) {
                    self.regions.iter().any(|region| {
                        code.trim().eq_ignore_ascii_case(region)
                            || code
                                .trim()
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
                };
            // Parsed cities are exact (ASCII case-insensitive); the same tuple's
            // raw evidence also supports literal substrings. Non-ASCII characters
            // are preserved, not transliterated or case-folded by SQLite lower().
            let city_match = self.cities.is_empty()
                || self.cities.iter().any(|target| {
                    let target = target.trim();
                    loc.city
                        .as_deref()
                        .is_some_and(|city| city.trim().eq_ignore_ascii_case(target))
                        || loc
                            .raw
                            .to_ascii_lowercase()
                            .contains(&target.to_ascii_lowercase())
                        || (self.location_include_unknown
                            && loc
                                .city
                                .as_deref()
                                .is_none_or(|city| city.trim().is_empty()))
                });
            country_match && region_match && city_match
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
            let unknown = match self.experience_match_mode {
                ExperienceMatchMode::AdvertisedRangeOverlap => min.is_none() && max.is_none(),
                ExperienceMatchMode::RequiredMinimum => min.is_none(),
            };
            if unknown {
                if !self.experience_include_unknown {
                    reasons.push(RejectionReason::Experience);
                }
            } else if match self.experience_match_mode {
                ExperienceMatchMode::AdvertisedRangeOverlap => {
                    self.experience_min
                        .is_some_and(|lo| max.is_some_and(|hi| hi < lo))
                        || self
                            .experience_max
                            .is_some_and(|hi| min.is_some_and(|lo| lo > hi))
                }
                ExperienceMatchMode::RequiredMinimum => {
                    self.experience_min
                        .is_some_and(|lo| min.is_some_and(|value| value < lo))
                        || self
                            .experience_max
                            .is_some_and(|hi| min.is_some_and(|value| value > hi))
                }
            } {
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
    pub fn assess(
        &self,
        event: &WorkerEvent,
        started: chrono::DateTime<chrono::Utc>,
        evidence: &MatchingEvidence,
    ) -> MatchingAssessment {
        let WorkerEvent::Job {
            title,
            company,
            location,
            description_text,
            role_type,
            date_posted,
            salary,
            experience,
            sponsorship,
            work_auth,
            clearance,
            locations,
            ..
        } = event
        else {
            return MatchingAssessment {
                rejected: vec![RejectionReason::InvalidCandidate],
                unresolved: vec![],
            };
        };
        let context = JobFilterContext {
            title,
            company,
            location,
            description: description_text.as_deref().unwrap_or(""),
            role_type: *role_type,
        };
        let failed = self.rejection_reasons_at(event, started);
        let mut assessment = MatchingAssessment::default();
        if let Some(keywords) = &self.keywords {
            for (reason, decision) in keywords.decisions(&context, &evidence.fields) {
                assessment.check(reason, decision);
            }
        }
        let active = [
            (RejectionReason::PostingDate, self.posting_date.is_some()),
            (RejectionReason::Compensation, self.compensation.is_some()),
            (RejectionReason::RoleType, !self.role_types.is_empty()),
            (RejectionReason::Workplace, !self.workplaces.is_empty()),
            (
                RejectionReason::Experience,
                self.experience_min.is_some() || self.experience_max.is_some(),
            ),
            (
                RejectionReason::NewGrad,
                matches!(self.new_grad.as_deref(), Some("only" | "exclude")),
            ),
            (RejectionReason::Sponsorship, !self.sponsorship.is_empty()),
            (
                RejectionReason::WorkAuthorization,
                !self.work_auth.is_empty(),
            ),
            (RejectionReason::Clearance, !self.clearance.is_empty()),
            (
                RejectionReason::Geography,
                !self.countries.is_empty() || !self.regions.is_empty() || !self.cities.is_empty(),
            ),
        ];
        for (reason, active) in active {
            if !active {
                continue;
            }
            let allowed_unknown = evidence.opportunity_exhausted
                && match reason {
                    RejectionReason::PostingDate => {
                        date_posted.is_none()
                            && self
                                .posting_date
                                .as_ref()
                                .is_some_and(|rule| rule.include_unknown)
                    }
                    RejectionReason::Compensation => {
                        salary.is_none()
                            && self
                                .compensation
                                .as_ref()
                                .is_some_and(|rule| rule.include_unknown)
                    }
                    RejectionReason::Experience => {
                        experience.as_ref().is_none_or(|e| {
                            e.min_years.is_none()
                                && (self.experience_match_mode
                                    == ExperienceMatchMode::RequiredMinimum
                                    || e.max_years.is_none())
                        }) && self.experience_include_unknown
                    }
                    RejectionReason::Sponsorship => {
                        sponsorship
                            .as_ref()
                            .is_none_or(|s| s.value == TriState::Unknown)
                            && self.sponsorship.contains(&TriState::Unknown)
                    }
                    RejectionReason::WorkAuthorization => {
                        work_auth
                            .as_ref()
                            .is_none_or(|s| s.value == WorkAuthRequirement::Unknown)
                            && self.work_auth.contains(&WorkAuthRequirement::Unknown)
                    }
                    RejectionReason::Clearance => {
                        clearance
                            .as_ref()
                            .is_none_or(|s| s.value == TriState::Unknown)
                            && self.clearance.contains(&TriState::Unknown)
                    }
                    RejectionReason::Geography => {
                        !crate::enrich::location::has_geographic_value(locations)
                            && self.location_include_unknown
                    }
                    RejectionReason::Workplace => self.workplaces.contains(&WorkplaceType::Unknown),
                    _ => false,
                };
            let known_geography = reason == RejectionReason::Geography
                && evidence.fields.contains(&FilterField::Location)
                && (crate::enrich::location::has_geographic_value(locations)
                    || self.cities.iter().any(|city| {
                        locations.iter().any(|l| {
                            l.raw
                                .to_ascii_lowercase()
                                .contains(&city.trim().to_ascii_lowercase())
                        })
                    }))
                && (evidence.opportunity_exhausted || !failed.contains(&reason));
            assessment.check(
                reason,
                ((evidence.knows(reason)
                    && !(reason == RejectionReason::Experience
                        && self.experience_match_mode == ExperienceMatchMode::RequiredMinimum
                        && experience.as_ref().is_none_or(|e| e.min_years.is_none())
                        && !evidence.opportunity_exhausted))
                    || allowed_unknown
                    || known_geography)
                    .then(|| !failed.contains(&reason)),
            );
        }
        assessment
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
                overlap.push(match self.experience_match_mode {
                    ExperienceMatchMode::AdvertisedRangeOverlap => {
                        "(experience_max IS NULL OR experience_max >= ?)"
                    }
                    ExperienceMatchMode::RequiredMinimum => "experience_min >= ?",
                });
                bindings.push(Bind::Integer(lo));
            }
            if let Some(hi) = self.experience_max {
                overlap.push("(experience_min IS NULL OR experience_min <= ?)");
                bindings.push(Bind::Integer(hi));
            }
            let known = match self.experience_match_mode {
                ExperienceMatchMode::AdvertisedRangeOverlap => {
                    "(experience_min IS NOT NULL OR experience_max IS NOT NULL)"
                }
                ExperienceMatchMode::RequiredMinimum => "(experience_min IS NOT NULL)",
            };
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
        if !self.countries.is_empty() || !self.regions.is_empty() || !self.cities.is_empty() {
            let mut geo = Vec::new();
            // SQLite trim's explicit character set mirrors Rust str::trim.
            let trim = |column: &str| {
                format!("NULLIF(trim({column},char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288)), '')")
            };
            let city_value = trim("jl.city");
            let region_value = trim("jl.region_code");
            let country_value = trim("jl.country_code");
            let country = format!("coalesce({country_value}, CASE WHEN instr({region_value},'-') > 0 THEN substr({region_value},1,instr({region_value},'-')-1) END)");
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
                    let condition = format!("(upper({region_value}) = ? OR substr(upper({region_value}), instr({region_value}, '-') + 1) = ?)");
                    bindings.push(Bind::Text(region.to_ascii_uppercase()));
                    bindings.push(Bind::Text(region.to_ascii_uppercase()));
                    parts.push(if self.location_include_unknown {
                        if let Some((prefix,_)) = region.split_once('-') {
                            bindings.push(Bind::Text(prefix.to_ascii_uppercase()));
                            format!("({condition} OR ({region_value} IS NULL AND ({country} IS NULL OR upper({country}) = ?)))")
                        } else { format!("({condition} OR {region_value} IS NULL)") }
                    } else { condition.to_string() });
                }
                geo.push(format!("({})", parts.join(" OR ")));
            }
            if !self.cities.is_empty() {
                let mut parts = Vec::new();
                for city in &self.cities {
                    let condition = format!(
                        "(lower({city_value}) = lower(?) OR instr(lower(jl.raw), lower(?)) > 0)"
                    );
                    bindings.push(Bind::Text(city.trim().into()));
                    bindings.push(Bind::Text(city.trim().into()));
                    parts.push(if self.location_include_unknown {
                        format!("({condition} OR {city_value} IS NULL)")
                    } else {
                        condition
                    });
                }
                geo.push(format!("({})", parts.join(" OR ")));
            }
            let known = format!("({country_value} IS NOT NULL OR {region_value} IS NOT NULL OR {city_value} IS NOT NULL)");
            let any_known = format!("EXISTS (SELECT 1 FROM job_locations jl_known WHERE jl_known.job_id=jobs.id AND {})", known.replace("jl.", "jl_known."));
            let exists = format!("EXISTS (SELECT 1 FROM job_locations jl WHERE jl.job_id=jobs.id AND ({known} OR NOT {any_known}) AND ({}))", geo.join(" AND "));
            clauses.push(if self.location_include_unknown {
                format!("({exists} OR NOT EXISTS (SELECT 1 FROM job_locations jl WHERE jl.job_id=jobs.id))")
            } else { exists });
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
            PostingRules {
                cities: vec!["  ".into()],
                ..Default::default()
            },
        ] {
            assert!(rules.validate().is_err());
        }
    }
    #[test]
    fn city_matching_and_combined_geography() {
        let city_loc = |city: &str, country: Option<&str>, raw: &str| ParsedLocation {
            raw: raw.into(),
            city: Some(city.into()),
            country_code: country.map(str::to_string),
            region_code: None,
            is_remote: false,
        };

        let rules = PostingRules {
            cities: vec!["Austin".into(), "Lisbon".into()],
            ..Default::default()
        };
        assert!(rules.geography_matches(&[city_loc("Austin", Some("US"), "Austin, TX")]));
        assert!(rules.geography_matches(&[city_loc("Lisbon", Some("PT"), "Lisbon, Portugal")]));
        assert!(!rules.geography_matches(&[city_loc("New York", Some("US"), "New York, NY")]));

        // Combined with countries: Austin (US) matches, Lisbon (PT) rejected
        let us_only = PostingRules {
            countries: vec!["US".into()],
            cities: vec!["Austin".into(), "Lisbon".into()],
            ..Default::default()
        };
        assert!(us_only.geography_matches(&[city_loc("Austin", Some("US"), "Austin, TX")]));
        assert!(!us_only.geography_matches(&[city_loc("Lisbon", Some("PT"), "Lisbon, Portugal")]));
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

#[cfg(test)]
mod incomplete_evidence_tests {
    use super::*;
    fn event(title: &str, description: Option<&str>) -> WorkerEvent {
        serde_json::from_value(serde_json::json!({"type":"job","source_id":"fixture","external_id":"1","title":title,"company":"Fixture","location":"Remote","role_type":"full_time","url":"https://fixture.test/1","description_text":description})).unwrap()
    }
    #[test]
    fn minimum_required_is_not_overlap_and_maximum_only_waits_for_details() {
        let mut rules = PostingRules {
            experience_min: Some(3),
            experience_max: Some(5),
            experience_match_mode: ExperienceMatchMode::RequiredMinimum,
            ..Default::default()
        };
        let clock = chrono::Utc::now();
        for (min, max, accepted) in [
            (Some(1), Some(5), false),
            (Some(3), None, true),
            (Some(6), None, false),
        ] {
            let mut candidate = event("Engineer", None);
            if let WorkerEvent::Job { experience, .. } = &mut candidate {
                *experience = Some(crate::enrich::experience::ExperienceClassification {
                    min_years: min,
                    max_years: max,
                    raw: None,
                    evidence: None,
                    is_new_grad: TriState::Unknown,
                });
            }
            assert_eq!(rules.matches_event_at(&candidate, clock), accepted);
            if min == Some(1) {
                let mut overlap = rules.clone();
                overlap.experience_match_mode = ExperienceMatchMode::AdvertisedRangeOverlap;
                assert!(overlap.matches_event_at(&candidate, clock));
            }
        }
        let mut candidate = event("Engineer", None);
        if let WorkerEvent::Job { experience, .. } = &mut candidate {
            *experience = Some(crate::enrich::experience::ExperienceClassification {
                min_years: None,
                max_years: Some(5),
                raw: None,
                evidence: None,
                is_new_grad: TriState::Unknown,
            });
        }
        for include_unknown in [false, true] {
            rules.experience_include_unknown = include_unknown;
            let pending = MatchingEvidence::from_event(&candidate, false);
            assert_eq!(
                rules.assess(&candidate, clock, &pending).unresolved,
                vec![RejectionReason::Experience]
            );
            let exhausted = MatchingEvidence::from_event(&candidate, true);
            assert_eq!(
                rules
                    .assess(&candidate, clock, &exhausted)
                    .rejected
                    .is_empty(),
                include_unknown
            );
            assert_eq!(rules.matches_event_at(&candidate, clock), include_unknown);
        }
        if let WorkerEvent::Job {
            experience: Some(experience),
            ..
        } = &mut candidate
        {
            experience.min_years = Some(1);
        }
        let hydrated = MatchingEvidence::from_event(&candidate, false);
        assert_eq!(
            rules.assess(&candidate, clock, &hydrated).rejected,
            vec![RejectionReason::Experience]
        );
        assert!(!rules.matches_event_at(&candidate, clock));
        let legacy: PostingRules =
            serde_json::from_value(serde_json::json!({"experience_min":3,"experience_max":5}))
                .unwrap();
        assert_eq!(
            legacy.experience_match_mode,
            ExperienceMatchMode::AdvertisedRangeOverlap
        );
        assert!(legacy.matches_event_at(&candidate, clock));
    }
    #[test]
    fn missing_description_is_unresolved_even_under_exclusion_and_not() {
        let evidence = MatchingEvidence {
            fields: vec![FilterField::Title, FilterField::RoleType],
            ..Default::default()
        };
        for value in [
            serde_json::json!({"matching":{"keywords":{"any":["Rust"],"fields":["description"]}}}),
            serde_json::json!({"matching":{"keywords":{"exclude":["Senior"],"fields":["description"]}}}),
            serde_json::json!({"query":"NOT (description LIKE '%Senior%')"}),
            serde_json::json!({"policy":{"none":[{"description":"%Senior%"}]}}),
        ] {
            let filters: crate::WorkerFilters = serde_json::from_value(value).unwrap();
            let evaluator = MatchingEvaluator::new(&filters).unwrap();
            let result = evaluator.assess(&event("Engineer", None), chrono::Utc::now(), &evidence);
            assert!(result.rejected.is_empty());
            assert_eq!(result.unresolved.len(), 1);
        }
    }
    #[test]
    fn known_title_can_reject_but_or_with_an_unresolved_field_cannot() {
        let evidence = MatchingEvidence {
            fields: vec![FilterField::Title, FilterField::RoleType],
            ..Default::default()
        };
        for (value, rejected) in [
            (
                serde_json::json!({"matching":{"keywords":{"any":["Engineer"],"fields":["title"]}}}),
                true,
            ),
            (
                serde_json::json!({"matching":{"keywords":{"any":["Engineer"],"fields":["title","description"]}}}),
                false,
            ),
            (
                serde_json::json!({"query":"title LIKE '%Engineer%' OR description LIKE '%Engineer%'"}),
                false,
            ),
            (
                serde_json::json!({"matching":{"role_types":["internship"]}}),
                true,
            ),
        ] {
            let filters: crate::WorkerFilters = serde_json::from_value(value).unwrap();
            let evaluator = MatchingEvaluator::new(&filters).unwrap();
            let result = evaluator.assess(&event("Sales", None), chrono::Utc::now(), &evidence);
            assert_eq!(!result.rejected.is_empty(), rejected);
        }
    }
    #[test]
    fn complete_assessments_reuse_boolean_predicates_and_explicit_unknowns() {
        let filters: crate::WorkerFilters = serde_json::from_value(serde_json::json!({"matching":{"keywords":{"any":["Engineer"],"all":["Rust"],"exclude":["Senior"],"fields":["title","description"]},"posting_date":{"last_hours":720,"timezone":"UTC","include_unknown":true},"compensation":{"min_minor":5000,"interval":"hour","currency":"USD","include_unknown":true},"role_types":["internship"],"workplaces":["remote"],"experience_min":2,"countries":["US"],"new_grad":"only","sponsorship":["yes"],"clearance":["no"],"work_auth":["citizen_only"]}})).unwrap();
        for event in [
            event("Senior Engineer", None),
            event("Rust Engineer", Some("Rust")),
            event("Sales", Some("Senior")),
        ] {
            let evaluator = MatchingEvaluator::new(&filters).unwrap();
            let started = chrono::DateTime::parse_from_rfc3339("2026-10-16T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc);
            let assessment = evaluator.assess(&event, started, &MatchingEvidence::complete());
            assert!(assessment.unresolved.is_empty());
            assert_eq!(
                assessment
                    .rejected
                    .into_iter()
                    .collect::<std::collections::BTreeSet<_>>(),
                evaluator
                    .rejection_reasons(&event, started)
                    .into_iter()
                    .collect()
            );
        }
    }
}

#[cfg(test)]
mod field_evidence_tests {
    use super::*;
    #[test]
    fn concrete_workplace_does_not_require_description_and_does_not_complete_unrelated_fields() {
        let event:WorkerEvent=serde_json::from_value(serde_json::json!({"type":"job","source_id":"fixture","external_id":"1","title":"Engineer","company":"Fixture","url":"https://fixture.test/1","location":"","role_type":"unknown","workplace_type":"remote"})).unwrap();
        let evidence = MatchingEvidence::from_event(&event, true);
        for (rule, expected) in [
            (serde_json::json!({"workplaces":["remote"]}), vec![]),
            (
                serde_json::json!({"workplaces":["onsite"]}),
                vec![RejectionReason::Workplace],
            ),
        ] {
            let rules: PostingRules = serde_json::from_value(rule).unwrap();
            let assessed = rules.assess(&event, chrono::Utc::now(), &evidence);
            assert!(assessed.unresolved.is_empty());
            assert_eq!(assessed.rejected, expected);
        }
        let rules:PostingRules=serde_json::from_value(serde_json::json!({"countries":["US"],"keywords":{"any":["Rust"],"fields":["description"]}})).unwrap();
        let assessed = rules.assess(&event, chrono::Utc::now(), &evidence);
        assert_eq!(
            assessed.unresolved,
            vec![RejectionReason::KeywordAny, RejectionReason::Geography]
        );
        assert!(assessed.rejected.is_empty());
    }
}
