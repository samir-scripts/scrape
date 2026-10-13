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
pub use html::to_plain_text;
pub use location::ParsedLocation;
pub use salary::Salary;
pub use workplace::classify_workplace;

/// Current version of the enrichment extraction engine. Bump this when heuristics or classifiers change.
pub const ENRICHMENT_VERSION: i32 = 2;
