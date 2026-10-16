use crate::client::HttpClient;
use scrape_core::WorkerEvent;
use scraper::{Html, Selector};
use serde_json::Value;
use std::time::Duration;
use url::Url;

/// Branding is optional: one bounded posting-page lookup per ATS source/run.
pub struct CompanyBranding {
    company: String,
    logo: Option<String>,
    warning: Option<String>,
}

impl CompanyBranding {
    pub async fn fetch(client: &HttpClient, page_url: &str, company: &str) -> Self {
        let mut warning = None;
        let logo = match tokio::time::timeout(
            Duration::from_secs(4),
            client.fetch_text(page_url, true),
        )
        .await
        {
            Ok(Ok(html)) => extract_company_logo(&html, page_url, company),
            result => {
                let message = format!("Company branding page unavailable for {page_url}: {result:?}. The logo stays unknown when no verified asset is available.");
                tracing::warn!("{message}");
                warning = Some(message);
                None
            }
        };
        Self {
            company: company.into(),
            logo,
            warning,
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
            ..
        } = &mut event
        {
            if company == &self.company && company_icon_url.is_none() {
                *company_icon_url = self.logo.clone();
            }
        }
        event
    }
}

pub fn logo_url(value: &Value, page_url: &str) -> Option<String> {
    let raw = value.as_str().or_else(|| value.get("url")?.as_str())?;
    if raw.trim().is_empty() {
        return None;
    }
    let url = Url::parse(page_url).ok()?.join(raw.trim()).ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none())
    .then(|| url.to_string())
}

/// Accept employer-labelled images or JobPosting employer metadata, never a guessed domain
/// or the recruiting platform's own favicon/social preview.
pub fn extract_company_logo(html: &str, page_url: &str, company: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let scripts = Selector::parse("script[type='application/ld+json']").unwrap();
    for script in document.select(&scripts) {
        if let Ok(value) = serde_json::from_str::<Value>(&script.inner_html()) {
            if let Some(logo) = job_posting_logo(&value, page_url, company) {
                return Some(logo);
            }
        }
    }
    let images = Selector::parse("img[alt][src]").unwrap();
    let company = company.trim().to_lowercase();
    if company.is_empty() {
        return None;
    }
    for image in document.select(&images) {
        let alt = image.value().attr("alt")?.trim().to_lowercase();
        if alt == company || alt == format!("{company} logo") || alt == format!("{company}'s logo")
        {
            if let Some(url) = logo_url(&Value::String(image.value().attr("src")?.into()), page_url)
            {
                return Some(url);
            }
        }
    }
    None
}

fn job_posting_logo(value: &Value, page_url: &str, company: &str) -> Option<String> {
    if let Some(values) = value.as_array() {
        return values
            .iter()
            .find_map(|value| job_posting_logo(value, page_url, company));
    }
    let kind = value.get("@type");
    let is_job = kind.is_some_and(|kind| {
        kind.as_str() == Some("JobPosting")
            || kind
                .as_array()
                .is_some_and(|types| types.iter().any(|t| t == "JobPosting"))
    });
    let employer_matches = value
        .pointer("/hiringOrganization/name")
        .and_then(Value::as_str)
        .is_none_or(|name| name.trim().eq_ignore_ascii_case(company.trim()));
    if is_job && employer_matches {
        if let Some(logo) = value
            .pointer("/hiringOrganization/logo")
            .and_then(|value| logo_url(value, page_url))
        {
            return Some(logo);
        }
    }
    value
        .get("@graph")
        .and_then(|graph| job_posting_logo(graph, page_url, company))
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
        assert_eq!(extract_company_logo(r#"<script type="application/ld+json">{"@graph":[{"@type":"JobPosting","hiringOrganization":{"logo":{"url":"/logo.svg"}}}]}</script>"#, page, "Acme").as_deref(), Some("https://jobs.example.com/logo.svg"));
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
