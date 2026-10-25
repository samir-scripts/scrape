pub mod eligibility;
pub mod experience;
pub mod html;
pub mod location;
pub mod salary;
pub mod workplace;

pub use eligibility::{
    classify_clearance, classify_sponsorship, classify_work_auth, ClassifiedClearance,
    ClassifiedSponsorship, ClassifiedWorkAuth,
};
pub use experience::{extract_experience, ExperienceClassification};
pub use html::{
    normalize_description, normalize_description_with_format, to_plain_text, DescriptionBlock,
    DescriptionFormat, DescriptionInline, JobDescription,
};
pub use location::ParsedLocation;
pub use salary::{CompensationFilter, CompensationMatchMode, CompensationRange, Salary};
pub use workplace::classify_workplace;

/// Current version of the enrichment extraction engine. Bump this when heuristics or classifiers change.
pub const ENRICHMENT_VERSION: i32 = 4;
