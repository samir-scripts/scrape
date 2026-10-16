use crate::client::HttpClient;
use scrape_core::{
    branding::{LogoEvidence, LogoMethod},
    WorkerEvent, WorkerSource,
};
use scraper::{Html, Selector};
use serde_json::Value;
use std::time::Duration;
use url::Url;

pub fn employer_hint(source: &WorkerSource) -> Option<&str> {
    source
        .config
        .get("__company")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}
pub fn employer_name(source: &WorkerSource) -> String {
    employer_hint(source).unwrap_or("Unknown company").into()
}
fn platform(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "greenhouse" | "lever" | "ashby" | "workday" | "recruiting platform"
    )
}
/// One evidence resolver, shared by ATS pages, structured data and custom sources.
pub struct CompanyBranding {
    company: String,
    evidence: Option<LogoEvidence>,
    warning: Option<String>,
}
impl CompanyBranding {
    pub async fn fetch(client: &HttpClient, page_url: &str, company: &str) -> Self {
        match tokio::time::timeout(Duration::from_secs(4),client.fetch_branding_page(page_url)).await {
            Ok(Ok((html, actual_page))) => {
                let actual_company=if company.trim().is_empty() { page_employer(&html).unwrap_or_default() } else { company.into() };
                Self { evidence: resolve_page(&html,&actual_page,&actual_company,None), company:actual_company,warning:None }
            }
            result => Self { company:company.into(),evidence:None,warning:Some(format!("Company branding page unavailable for {page_url}: {result:?}. Retain any previously verified logo.")) }
        }
    }
    pub async fn fetch_source(
        client: &HttpClient,
        page_url: &str,
        company: &str,
        source: &WorkerSource,
    ) -> Self {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
        let original = Self::fetch(client, page_url, company).await;
        if original.evidence.is_some() {
            return original;
        }
        let Some(board) = source
            .config
            .get("url")
            .and_then(Value::as_str)
            .filter(|url| *url != page_url && scrape_core::branding::http_url(url))
        else {
            return original;
        };
        if tokio::time::Instant::now() >= deadline {
            return original;
        }
        let employer = if original.company.is_empty() {
            company
        } else {
            &original.company
        };
        match tokio::time::timeout(Duration::from_secs(4), Self::fetch(client, board, employer))
            .await
        {
            Ok(fallback) if fallback.evidence.is_some() => fallback,
            _ => original,
        }
    }
    pub fn warning(&self, source_id: &str) -> Option<WorkerEvent> {
        self.warning.as_ref().map(|message| WorkerEvent::Warning {
            source_id: source_id.into(),
            message: message.clone(),
            eligibility_incomplete: false,
            termination: None,
        })
    }
    pub fn apply(&self, mut event: WorkerEvent) -> WorkerEvent {
        if let WorkerEvent::Job {
            company,
            company_icon_url,
            company_logo,
            ..
        } = &mut event
        {
            if company == "Unknown company" && !self.company.is_empty() {
                *company = self.company.clone();
            }
            if let Some(evidence) = self.evidence.as_ref().filter(|evidence| {
                company.trim().eq_ignore_ascii_case(self.company.trim())
                    && company_logo
                        .as_ref()
                        .is_none_or(|current| evidence.method < current.method)
            }) {
                *company_icon_url = Some(evidence.url.clone());
                *company_logo = Some(evidence.clone());
            }
        }
        event
    }
}
pub fn logo_url(value: &Value, page_url: &str) -> Option<String> {
    let raw = value
        .as_str()
        .or_else(|| value.get("url")?.as_str())?
        .trim();
    if raw.is_empty() {
        return None;
    }
    let resolved = Url::parse(page_url).ok()?.join(raw).ok()?.to_string();
    scrape_core::branding::http_url(&resolved).then_some(resolved)
}
pub fn structured_logo(value: &Value, page_url: &str, company: &str) -> Option<LogoEvidence> {
    if !scrape_core::branding::http_url(page_url) || company.trim().is_empty() || platform(company)
    {
        return None;
    }
    if let Some(array) = value.as_array() {
        return array
            .iter()
            .find_map(|v| structured_logo(v, page_url, company));
    }
    let is_job = value.get("@type").is_some_and(|t| {
        t == "JobPosting"
            || t.as_array()
                .is_some_and(|a| a.iter().any(|t| t == "JobPosting"))
    });
    if is_job
        && value
            .pointer("/hiringOrganization/name")
            .and_then(Value::as_str)
            .is_some_and(|name| name.trim().eq_ignore_ascii_case(company.trim()))
    {
        if let Some(url) = value
            .pointer("/hiringOrganization/logo")
            .and_then(|v| logo_url(v, page_url))
        {
            return Some(LogoEvidence {
                url,
                page_url: page_url.into(),
                method: LogoMethod::JobPosting,
            });
        }
    }
    value
        .get("@graph")
        .and_then(|v| structured_logo(v, page_url, company))
}
fn page_employer(html: &str) -> Option<String> {
    fn name(value: &Value) -> Option<String> {
        if let Some(a) = value.as_array() {
            return a.iter().find_map(name);
        }
        if value.get("@type").is_some_and(|t| {
            t == "JobPosting"
                || t.as_array()
                    .is_some_and(|a| a.iter().any(|t| t == "JobPosting"))
        }) {
            if let Some(n) = value
                .pointer("/hiringOrganization/name")
                .and_then(Value::as_str)
                .filter(|n| !n.trim().is_empty() && !platform(n))
            {
                return Some(n.trim().into());
            }
        }
        value.get("@graph").and_then(name)
    }
    let doc = Html::parse_document(html);
    doc.select(&Selector::parse("script[type='application/ld+json']").unwrap())
        .find_map(|s| {
            serde_json::from_str::<Value>(&s.inner_html())
                .ok()
                .and_then(|v| name(&v))
        })
}
pub fn resolve_page(
    html: &str,
    page_url: &str,
    company: &str,
    configured: Option<&str>,
) -> Option<LogoEvidence> {
    if !scrape_core::branding::http_url(page_url)
        || company.trim().is_empty()
        || company == "Unknown company"
        || platform(company)
    {
        return None;
    }
    let document = Html::parse_document(html);
    for script in document.select(&Selector::parse("script[type='application/ld+json']").unwrap()) {
        if let Ok(value) = serde_json::from_str::<Value>(&script.inner_html()) {
            if let Some(logo) = structured_logo(&value, page_url, company) {
                return Some(logo);
            }
        }
    }
    let normalized = company.trim().to_lowercase();
    for image in document.select(&Selector::parse("img[alt][src]").unwrap()) {
        let alt = image
            .value()
            .attr("alt")
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        if alt == normalized
            || alt == format!("{normalized} logo")
            || alt == format!("{normalized}'s logo")
        {
            if let Some(url) = logo_url(
                &Value::String(image.value().attr("src").unwrap_or_default().into()),
                page_url,
            ) {
                return Some(LogoEvidence {
                    url,
                    page_url: page_url.into(),
                    method: LogoMethod::EmployerLabel,
                });
            }
        }
    }
    let selector = Selector::parse(configured?).ok()?;
    let url = document
        .select(&selector)
        .find_map(|e| logo_url(&Value::String(e.value().attr("src")?.into()), page_url))?;
    Some(LogoEvidence {
        url,
        page_url: page_url.into(),
        method: LogoMethod::ConfiguredSelector,
    })
}
pub fn extract_company_logo(html: &str, page_url: &str, company: &str) -> Option<String> {
    resolve_page(html, page_url, company, None).map(|e| e.url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_greenhouse_branding_uses_the_employers_real_asset() {
        let logo = extract_company_logo(
            include_str!("../../tests/fixtures/greenhouse_branding.html"),
            "https://job-boards.greenhouse.io/cloudflare/jobs/8097321",
            "Cloudflare",
        );
        assert_eq!(logo.as_deref(), Some("https://recruiting.cdn.greenhouse.io/external_greenhouse_job_boards/logos/000/007/685/original/Image_20221024_121529_116.png?1666631611"));
    }

    #[test]
    fn extracts_only_employer_branding_and_resolves_relative_urls() {
        let page = "https://jobs.example.com/acme/123";
        assert_eq!(extract_company_logo(r#"<img alt="Recruiting platform Logo" src="/platform.png"><img alt="Acme Logo" src="/employer.png">"#, page, "Acme").as_deref(), Some("https://jobs.example.com/employer.png"));
        assert_eq!(
            extract_company_logo(
                r#"<link rel="icon" href="/platform.ico"><meta property="og:image" content="/banner.png">"#,
                page,
                "Acme"
            ),
            None
        );
        assert_eq!(
            extract_company_logo(
                r#"<img alt="Acme Logo" src="javascript:alert(1)">"#,
                page,
                "Acme"
            ),
            None
        );
        assert_eq!(extract_company_logo(r#"<script type="application/ld+json">{"@graph":[{"@type":"JobPosting","hiringOrganization":{"name":"Acme","logo":{"url":"/logo.svg"}}}]}</script>"#, page, "Acme").as_deref(), Some("https://jobs.example.com/logo.svg"));
        assert_eq!(
            extract_company_logo(
                r#"<script type="application/ld+json">{"@type":"JobPosting","hiringOrganization":{"name":"Other company","logo":"/other.svg"}}</script>"#,
                page,
                "Acme"
            ),
            None
        );
    }
}

#[cfg(test)]
mod priority_tests {
    use super::*;
    #[test]
    fn matching_structured_employer_precedes_labels_and_explicit_selectors() {
        let page = "https://fixture.test/careers/job";
        let html = r#"<script type="application/ld+json">{"@type":"JobPosting","hiringOrganization":{"name":"Acme","logo":"../structured.svg"}}</script><img alt="Acme Logo" src="/label.svg"><img class="custom" src="/custom.svg">"#;
        let logo = resolve_page(html, page, "Acme", Some(".custom")).unwrap();
        assert_eq!(logo.method, LogoMethod::JobPosting);
        assert_eq!(logo.url, "https://fixture.test/structured.svg");
        assert!(resolve_page(html, page, "Engineering jobs", None).is_none());
        assert!(resolve_page(
            html,
            "https://user:password@fixture.test/jobs",
            "Acme",
            None
        )
        .is_none());
        assert!(resolve_page(r#"<script type="application/ld+json">{"@type":"JobPosting","hiringOrganization":{"logo":"/nameless.svg"}}</script>"#,page,"Acme",None).is_none());
        assert!(resolve_page(
            "<img alt='Greenhouse Logo' src='/greenhouse.svg'>",
            page,
            "Greenhouse",
            None
        )
        .is_none());
        assert!(resolve_page(
            "<img alt='Acme Logo' src='https://user:password@fixture.test/logo.png'>",
            page,
            "Acme",
            None
        )
        .is_none());
    }
}
