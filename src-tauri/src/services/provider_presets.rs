//! Reusable company source choices, independent of production workflow profiles.
use super::template_catalog::{CompanyPreset, TemplateReadiness};
use super::workflow::WorkflowSourceSpec;
use crate::error::Result;
use serde_json::{json, Value};

fn preset(
    id: &str,
    name: &str,
    readiness: TemplateReadiness,
    description: &str,
    extractor: Value,
) -> Result<CompanyPreset> {
    let provider = match extractor["kind"].as_str().unwrap() {
        "greenhouse" => "Greenhouse",
        "ashby" => "Ashby",
        "lever" => "Lever",
        "workday" => "Workday",
        _ => "Careers",
    };
    let source: WorkflowSourceSpec = serde_json::from_value(json!({
        "id": id, "label": format!("{name} — {provider}"),
        "enabled": readiness == TemplateReadiness::Ready, "employer_hint": name,
        "extractor": extractor
    }))?;
    Ok(CompanyPreset {
        id: id.into(),
        name: name.into(),
        source,
        readiness,
        description: description.into(),
        logo: crate::db::branding::resolve_curated_logo(id),
    })
}

pub fn companies() -> Result<Vec<CompanyPreset>> {
    let ready = "Direct employer board. Bounded pagination and provider evidence apply; live availability varies. Preview before saving.";
    let review = "Static selector source, disabled for review. Google URL resolution and Apple selectors/pagination still need verification; not verified full-site ingestion. Review and enable explicitly.";
    let setup = "Bundled script protocol 2 source, disabled. Set an absolute Node.js executable, save, review and trust the script, then enable explicitly. Pagination coverage remains incomplete.";
    Ok(vec![
        preset(
            "cloudflare",
            "Cloudflare",
            TemplateReadiness::Ready,
            ready,
            json!({"kind":"greenhouse","url":"https://boards.greenhouse.io/cloudflare"}),
        )?,
        preset(
            "bitwarden",
            "Bitwarden",
            TemplateReadiness::Ready,
            ready,
            json!({"kind":"greenhouse","url":"https://boards.greenhouse.io/bitwarden"}),
        )?,
        preset(
            "linear",
            "Linear",
            TemplateReadiness::Ready,
            ready,
            json!({"kind":"ashby","url":"https://jobs.ashbyhq.com/linear"}),
        )?,
        preset(
            "sep",
            "SEP",
            TemplateReadiness::Ready,
            ready,
            json!({"kind":"lever","url":"https://jobs.lever.co/sep"}),
        )?,
        preset(
            "google",
            "Google",
            TemplateReadiness::NeedsReview,
            review,
            json!({
                "kind":"selector", "mode":"static", "url":"https://www.google.com/about/careers/applications/jobs/results/",
                "job_container":"li.lLd3Je", "title_selector":"h3.QJPWVe", "url_selector":"a[href*=\"jobs/results/\"]",
                "location_selector":"span.r0wTof", "description_selector":"div.Xsxa1e", "next_page_selector":"a[aria-label=\"Go to next page\"]"
            }),
        )?,
        preset(
            "apple",
            "Apple",
            TemplateReadiness::NeedsReview,
            review,
            json!({
                "kind":"selector", "mode":"static", "url":"https://jobs.apple.com/en-us/search",
                "job_container":"ul#search-job-list > li.rc-accordion-item", "title_selector":"h3 a.link-inline", "url_selector":"a.link-inline",
                "location_selector":"div.job-title-location span:not(.a11y)", "date_selector":"span.job-posted-date",
                "description_selector":"p.text-align-start", "next_page_selector":"link[rel=\"next\"]"
            }),
        )?,
        preset(
            "netflix",
            "Netflix",
            TemplateReadiness::SetupRequired,
            setup,
            json!({
                "kind":"selector", "mode":"script", "url":"https://explore.jobs.netflix.net/api/apply/v2/jobs?domain=netflix.com&start=0&num=10",
                "script":{"runtime":"js","code":include_str!("../../../crates/scrape-worker/examples/extractors/netflix_extractor.js")}
            }),
        )?,
        preset(
            "amazon",
            "Amazon",
            TemplateReadiness::SetupRequired,
            setup,
            json!({
                "kind":"selector", "mode":"script", "url":"https://www.amazon.jobs/en/search.json?offset=0&result_limit=10",
                "script":{"runtime":"js","code":include_str!("../../../crates/scrape-worker/examples/extractors/amazon_extractor.js")}
            }),
        )?,
        preset(
            "nvidia",
            "NVIDIA",
            TemplateReadiness::Ready,
            ready,
            json!({"kind":"workday","url":"https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite"}),
        )?,
    ])
}
