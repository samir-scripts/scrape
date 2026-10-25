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
#[derive(Clone)]
pub struct CompanyBranding {
    pub company: String,
    pub evidence: Option<LogoEvidence>,
    pub warning: Option<String>,
}
impl CompanyBranding {
    async fn fetch_until(
        client: &HttpClient,
        page_url: &str,
        company: &str,
        deadline: tokio::time::Instant,
    ) -> Self {
        match tokio::time::timeout_at(deadline, client.fetch_branding_page(page_url)).await {
            Ok(Ok((html, actual_page))) => {
                let actual_company = if company.trim().is_empty() { page_employer(&html).unwrap_or_default() } else { company.into() };
                Self { evidence: resolve_page(&html, &actual_page, &actual_company, None), company: actual_company, warning: None }
            }
            result => Self { company: company.into(), evidence: None, warning: Some(format!("Company branding page unavailable for {page_url}: {result:?}. Retain any previously verified logo.")) }
        }
    }
    pub async fn fetch_source(
        client: &HttpClient,
        page_url: &str,
        company: &str,
        source: &WorkerSource,
    ) -> Self {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
        let original = Self::fetch_until(client, page_url, company, deadline).await;
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
        let fallback = Self::fetch_until(client, board, employer, deadline).await;
        if fallback.evidence.is_some() {
            fallback
        } else {
            original
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
        .or_else(|| value.get("url").and_then(Value::as_str))
        .or_else(|| value.get("contentUrl").and_then(Value::as_str))?
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
/// Read only a JSON object literal, never JavaScript expressions or executable code.
fn ashby_app_data(script: &str) -> Option<Value> {
    let (_, assignment) = script.split_once("window.__appData")?;
    let json = assignment.trim_start().strip_prefix('=')?.trim_start();
    if !json.starts_with('{') {
        return None;
    }
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in json.bytes().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' => depth += 1,
                b'}' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return serde_json::from_str(&json[..=index]).ok();
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn ashby_theme_logo(document: &Html, page_url: &str, company: &str) -> Option<LogoEvidence> {
    if Url::parse(page_url).ok()?.host_str() != Some("jobs.ashbyhq.com") {
        return None;
    }
    // Match the HTTP page cap even for callers that supply a pre-parsed document.
    let mut remaining: usize = 2 * 1024 * 1024;
    for script in document.select(&Selector::parse("script").unwrap()) {
        for text in script.text() {
            remaining = remaining.checked_sub(text.len())?;
            let Some(data) = ashby_app_data(text) else {
                continue;
            };
            let organization = data.get("organization")?;
            let name = organization.get("name")?.as_str()?;
            if !name.trim().eq_ignore_ascii_case(company.trim()) {
                return None;
            }
            let organization_id = organization.get("organizationId")?.as_str()?;
            if organization_id.is_empty() {
                return None;
            }
            let theme = organization.get("theme")?;
            for field in ["logoSquareImageUrl", "logoWordmarkImageUrl"] {
                let Some(raw) = theme.get(field).and_then(Value::as_str) else {
                    continue;
                };
                let Ok(url) = Url::parse(raw) else {
                    continue;
                };
                if url.scheme() != "https"
                    || url.host_str() != Some("app.ashbyhq.com")
                    || !scrape_core::branding::http_url(raw)
                {
                    continue;
                }
                let path = url.path();
                let asset = path
                    .strip_prefix("/api/images/org-theme-logo/")
                    .or_else(|| path.strip_prefix("/api/images/org-theme-wordmark/"));
                if asset
                    .and_then(|path| path.split_once('/'))
                    .is_some_and(|(id, rest)| id == organization_id && !rest.is_empty())
                {
                    return Some(LogoEvidence {
                        url: url.into(),
                        page_url: page_url.into(),
                        method: LogoMethod::Provider,
                    });
                }
            }
        }
    }
    None
}

pub fn resolve_page(
    html: &str,
    page_url: &str,
    company: &str,
    configured: Option<&str>,
) -> Option<LogoEvidence> {
    resolve_document(&Html::parse_document(html), page_url, company, configured)
}
/// Reuse the caller's HTML5 tree when a custom source already parsed this page.
pub fn resolve_document_with_base(
    document: &Html,
    page_url: &str,
    base_url: &str,
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
    if let Some(logo) = ashby_theme_logo(document, page_url, company) {
        return Some(logo);
    }
    for script in document.select(&Selector::parse("script[type='application/ld+json']").unwrap()) {
        if let Ok(value) = serde_json::from_str::<Value>(&script.inner_html()) {
            if let Some(logo) = structured_logo(&value, base_url, company) {
                return Some(LogoEvidence {
                    url: logo.url,
                    page_url: page_url.into(),
                    method: logo.method,
                });
            }
        }
    }
    let normalized = company.trim().to_lowercase();
    for image in document.select(&Selector::parse("img[alt]").unwrap()) {
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
            if let Some(url) = image_url(image, base_url) {
                return Some(LogoEvidence {
                    url,
                    page_url: page_url.into(),
                    method: LogoMethod::EmployerLabel,
                });
            }
        }
    }
    for anchor in document.select(&Selector::parse("a[aria-label]").unwrap()) {
        let label = anchor
            .value()
            .attr("aria-label")
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        if label == normalized
            || label == format!("{normalized} home")
            || label == format!("{normalized} careers")
        {
            if let Some(img) = anchor.select(&Selector::parse("img").unwrap()).next() {
                if let Some(url) = image_url(img, base_url) {
                    return Some(LogoEvidence {
                        url,
                        page_url: page_url.into(),
                        method: LogoMethod::EmployerLabel,
                    });
                }
            }
        }
    }
    let selector = Selector::parse(configured?).ok()?;
    let url = document
        .select(&selector)
        .find_map(|e| image_url(e, base_url))?;
    Some(LogoEvidence {
        url,
        page_url: page_url.into(),
        method: LogoMethod::ConfiguredSelector,
    })
}
/// Only called after the surrounding markup has established employer identity.
fn image_url(image: scraper::ElementRef<'_>, base_url: &str) -> Option<String> {
    if let Some(srcset) = image.value().attr("srcset") {
        let candidates = srcset
            .split(',')
            .filter_map(|part| {
                let mut fields = part.split_whitespace();
                let raw = fields.next()?;
                let width = fields.next()?.strip_suffix('w')?.parse::<u32>().ok()?;
                if fields.next().is_some() || width == 0 {
                    return None;
                }
                Some((width, logo_url(&Value::String(raw.into()), base_url)?))
            })
            .collect::<Vec<_>>();
        if let Some((_, url)) = candidates
            .iter()
            .find(|(width, _)| *width >= 128)
            .or_else(|| candidates.iter().max_by_key(|(width, _)| *width))
        {
            return Some(url.clone());
        }
    }
    ["src", "data-src"].iter().find_map(|attribute| {
        logo_url(
            &Value::String(image.value().attr(attribute)?.into()),
            base_url,
        )
    })
}

pub fn resolve_document(
    document: &Html,
    page_url: &str,
    company: &str,
    configured: Option<&str>,
) -> Option<LogoEvidence> {
    let response_url = Url::parse(page_url).ok()?;
    let base = super::selector::resolve_document_base(document, &response_url);
    resolve_document_with_base(document, page_url, base.as_str(), company, configured)
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
    #[test]
    fn employer_lazy_assets_follow_document_base_without_aliasing_companies() {
        let html = r#"<base href="/assets/"><img alt="Acme Labs Logo" src="wrong.svg"><img alt="Acme Logo" srcset="javascript:bad 256w, small.webp 64w, medium.webp 128w, huge.webp 512w" data-src="fallback.webp">"#;
        let evidence = resolve_page(html, "https://fixture.test/jobs/1", "Acme", None).unwrap();
        assert_eq!(evidence.url, "https://fixture.test/assets/medium.webp");
        assert_eq!(evidence.page_url, "https://fixture.test/jobs/1");
        let structured = resolve_page(r#"<base href="/assets/"><script type="application/ld+json">{"@type":"JobPosting","hiringOrganization":{"name":"Acme","logo":{"@type":"ImageObject","contentUrl":"employer.webp"}}}</script>"#, "https://fixture.test/jobs/1", "Acme", None).unwrap();
        assert_eq!(structured.url, "https://fixture.test/assets/employer.webp");
        assert_eq!(structured.page_url, "https://fixture.test/jobs/1");
        assert_eq!(
            extract_company_logo(
                "<img alt='Acme Logo' data-src='../logo.webp'>",
                "https://fixture.test/jobs/1",
                "Acme"
            )
            .as_deref(),
            Some("https://fixture.test/logo.webp")
        );
        assert!(extract_company_logo(
            "<img alt='Acme Labs Logo' src='/wrong.webp'>",
            "https://fixture.test/jobs/1",
            "Acme"
        )
        .is_none());
    }

    fn ashby_theme_page(name: &str, id: &str, square: Value, wordmark: Value) -> String {
        let data = serde_json::json!({
            "organization": {
                "name": name,
                "organizationId": id,
                "theme": {
                    "logoSquareImageUrl": square,
                    "logoWordmarkImageUrl": wordmark
                }
            },
            "other": "Quoted braces } { and an escaped quote \" and backslash \\"
        });
        format!(
            "<link rel='icon' type='image/png' href='https://cdn.ashbyprd.com/cdn_assets/favicon.png'><script>window.__appData = {data}; window.after = true;</script>"
        )
    }

    #[test]
    fn ashby_theme_prefers_matching_employer_square_over_wordmark_and_favicon() {
        let square = "https://app.ashbyhq.com/api/images/org-theme-logo/org-123/theme/square.png";
        let wordmark =
            "https://app.ashbyhq.com/api/images/org-theme-wordmark/org-123/theme/wordmark.png";
        let page = "https://jobs.ashbyhq.com/acme/job-123";
        let html = ashby_theme_page("  aCmE  ", "org-123", square.into(), wordmark.into());
        let logo = resolve_page(&html, page, "Acme", None).unwrap();
        assert_eq!(logo.url, square);
        assert_eq!(logo.page_url, page);
        assert_eq!(logo.method, LogoMethod::Provider);
    }

    #[test]
    fn ashby_theme_uses_wordmark_when_square_is_absent_or_untrusted() {
        let wordmark =
            "https://app.ashbyhq.com/api/images/org-theme-wordmark/org-123/theme/wordmark.png";
        for square in [
            Value::Null,
            Value::String("https://evil.test/square.png".into()),
        ] {
            let html = ashby_theme_page("Acme", "org-123", square, wordmark.into());
            let logo = resolve_page(&html, "https://jobs.ashbyhq.com/acme", "Acme", None).unwrap();
            assert_eq!(logo.url, wordmark);
            assert_eq!(logo.method, LogoMethod::Provider);
        }
    }

    #[test]
    fn ashby_theme_rejects_wrong_employer_or_unscoped_assets() {
        let square = "https://app.ashbyhq.com/api/images/org-theme-logo/org-123/theme/square.png";
        let page = "https://jobs.ashbyhq.com/acme";
        let wrong_employer =
            ashby_theme_page("Other employer", "org-123", square.into(), Value::Null);
        assert!(resolve_page(&wrong_employer, page, "Acme", None).is_none());
        for asset in [
            "https://app.ashbyhq.com/api/images/org-theme-logo/other-org/theme/logo.png",
            "https://app.ashbyhq.com/api/images/org-theme-logo/org-123-extra/theme/logo.png",
            "https://app.ashbyhq.com/api/images/org-theme-logo/org-123",
            "https://app.ashbyhq.com/api/images/org-theme-logo/org-123/",
            "https://app.ashbyhq.com/api/images/other/org-123/logo.png",
            "https://evil.test/api/images/org-theme-logo/org-123/theme/logo.png",
            "https://app.ashbyhq.com.evil.test/api/images/org-theme-logo/org-123/theme/logo.png",
            "http://app.ashbyhq.com/api/images/org-theme-logo/org-123/theme/logo.png",
            "https://user@app.ashbyhq.com/api/images/org-theme-logo/org-123/theme/logo.png",
            "//app.ashbyhq.com/api/images/org-theme-logo/org-123/theme/logo.png",
            "javascript:alert(1)",
        ] {
            let html = ashby_theme_page("Acme", "org-123", asset.into(), Value::Null);
            assert!(resolve_page(&html, page, "Acme", None).is_none(), "{asset}");
        }
        let html = ashby_theme_page("Acme", "org-123", square.into(), Value::Null);
        for wrong_page in [
            "https://example.test/acme",
            "https://jobs.ashbyhq.com.evil.test/acme",
            "https://app.ashbyhq.com/acme",
        ] {
            assert!(resolve_page(&html, wrong_page, "Acme", None).is_none());
        }
    }

    #[test]
    fn ashby_payload_requires_bounded_valid_json_not_javascript() {
        for script in [
            "window.__appData = {",
            "window.__appData = {\"organization\":",
            "window.__appData = {'organization': {}};",
            "window.__appData = JSON.parse('{}');",
            "window.__appData = (() => ({organization: {}}))();",
            "window.__appData = null;",
            "window.__appData = {};",
            "window.__appData = {\"organization\":{\"name\":\"Acme\"}};",
        ] {
            let html = format!("<script>{script}</script>");
            assert!(resolve_page(&html, "https://jobs.ashbyhq.com/acme", "Acme", None).is_none());
        }
        let square = "https://app.ashbyhq.com/api/images/org-theme-logo/org-123/theme/logo.png";
        let html = format!(
            "<script>{}</script>{}",
            " ".repeat(2 * 1024 * 1024),
            ashby_theme_page("Acme", "org-123", square.into(), Value::Null)
        );
        assert!(resolve_page(&html, "https://jobs.ashbyhq.com/acme", "Acme", None).is_none());
    }

    #[test]
    fn generic_favicons_and_social_images_are_never_employer_evidence() {
        for markup in [
            "<link rel='icon' type='image/png' href='/favicon.png'>",
            "<link rel='shortcut icon' type='image/webp' href='/favicon.webp'>",
            "<link rel='apple-touch-icon' href='/apple-touch-icon.png'>",
            "<link rel='icon' href='/favicon.ico'>",
            "<meta property='og:image' content='/social.png'>",
        ] {
            for page in [
                "https://jobs.ashbyhq.com/acme",
                "https://example.test/jobs/1",
            ] {
                assert!(
                    resolve_page(markup, page, "Acme", None).is_none(),
                    "{markup}"
                );
            }
        }
    }

    #[test]
    fn ordinary_employer_evidence_remains_available_without_provider_metadata() {
        let page = "https://jobs.ashbyhq.com/acme/job";
        for (html, configured, method) in [
            (
                "<a aria-label='Acme careers'><img src='/employer.png'></a>",
                None,
                LogoMethod::EmployerLabel,
            ),
            (
                "<img class='configured-logo' src='/employer.png'>",
                Some(".configured-logo"),
                LogoMethod::ConfiguredSelector,
            ),
            (
                r#"<script type="application/ld+json">{"@type":"JobPosting","hiringOrganization":{"name":"Acme","logo":"/employer.png"}}</script>"#,
                None,
                LogoMethod::JobPosting,
            ),
        ] {
            let logo = resolve_page(html, page, "Acme", configured).unwrap();
            assert_eq!(logo.url, "https://jobs.ashbyhq.com/employer.png");
            assert_eq!(logo.method, method);
        }
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
