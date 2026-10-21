use crate::client::HttpClient;
use scrape_core::SourceKind;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectResult {
    pub kind: SourceKind,
    pub identifier: String,
    pub name: String,
}

pub async fn detect_source(client: &HttpClient, target_url: &str) -> DetectResult {
    let parsed = match Url::parse(target_url) {
        Ok(u) => u,
        Err(_) => {
            return DetectResult {
                kind: SourceKind::Selector,
                identifier: target_url.to_string(),
                name: "Custom Source".to_string(),
            }
        }
    };

    // 1. Direct URL inspection
    if let Some(host) = parsed.host_str() {
        if host.contains("greenhouse.io") {
            let segments: Vec<&str> = parsed
                .path_segments()
                .map(|s| s.collect())
                .unwrap_or_default();
            if let Some(token) = segments.first() {
                if !token.is_empty() {
                    return DetectResult {
                        kind: SourceKind::Greenhouse,
                        identifier: token.to_string(),
                        name: capitalize(token),
                    };
                }
            }
        }

        if host.contains("lever.co") {
            let segments: Vec<&str> = parsed
                .path_segments()
                .map(|s| s.collect())
                .unwrap_or_default();
            if let Some(slug) = segments.first() {
                if !slug.is_empty() {
                    return DetectResult {
                        kind: SourceKind::Lever,
                        identifier: slug.to_string(),
                        name: capitalize(slug),
                    };
                }
            }
        }

        if host.contains("ashbyhq.com") {
            let segments: Vec<&str> = parsed
                .path_segments()
                .map(|s| s.collect())
                .unwrap_or_default();
            if let Some(name) = segments.first() {
                if !name.is_empty() {
                    return DetectResult {
                        kind: SourceKind::Ashby,
                        identifier: name.to_string(),
                        name: capitalize(name),
                    };
                }
            }
        }

        if host.ends_with(".myworkdayjobs.com") {
            let tenant = host.split('.').next().unwrap_or("Workday");
            return DetectResult {
                kind: SourceKind::Workday,
                identifier: target_url.to_string(),
                name: capitalize(tenant),
            };
        }
    }

    // 2. Fetch page HTML and detect embeds / links / JSON-LD
    let html = match client.fetch_text(target_url, false).await {
        Ok(t) => t,
        Err(_) => {
            let host_name = parsed.host_str().unwrap_or("Career Page");
            return DetectResult {
                kind: SourceKind::Selector,
                identifier: target_url.to_string(),
                name: host_name.to_string(),
            };
        }
    };

    detect_from_html(&html, &parsed)
}

pub fn detect_from_html(html: &str, parsed: &Url) -> DetectResult {
    let company_name = extract_page_title_or_host(html, parsed);

    // Look for Greenhouse embed / links
    if let Some(token) = find_regex_match(html, r#"boards\.greenhouse\.io/([a-zA-Z0-9_\-]+)"#) {
        return DetectResult {
            kind: SourceKind::Greenhouse,
            identifier: token,
            name: company_name,
        };
    }
    if let Some(token) = find_regex_match(html, r#"job-boards\.greenhouse\.io/([a-zA-Z0-9_\-]+)"#) {
        return DetectResult {
            kind: SourceKind::Greenhouse,
            identifier: token,
            name: company_name,
        };
    }

    // Look for Lever embed / links
    if let Some(slug) = find_regex_match(html, r#"jobs\.lever\.co/([a-zA-Z0-9_\-]+)"#) {
        return DetectResult {
            kind: SourceKind::Lever,
            identifier: slug,
            name: company_name,
        };
    }

    // Look for Ashby embed / links
    if let Some(name) = find_regex_match(html, r#"jobs\.ashbyhq\.com/([a-zA-Z0-9_\-]+)"#) {
        return DetectResult {
            kind: SourceKind::Ashby,
            identifier: name,
            name: company_name,
        };
    }

    // Look for Workday embed / links
    if let Some(workday_url) = find_regex_match(
        html,
        r#"https://[a-zA-Z0-9_\-\.]+\.myworkdayjobs\.com/[a-zA-Z0-9_\-]+"#,
    ) {
        return DetectResult {
            kind: SourceKind::Workday,
            identifier: workday_url,
            name: company_name,
        };
    }

    // Look for JSON-LD JobPosting
    if html.contains("application/ld+json") && html.contains("JobPosting") {
        return DetectResult {
            kind: SourceKind::Jsonld,
            identifier: parsed.as_str().to_string(),
            name: company_name,
        };
    }

    // Fallback: Selector
    DetectResult {
        kind: SourceKind::Selector,
        identifier: parsed.as_str().to_string(),
        name: company_name,
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn extract_page_title_or_host(html: &str, url: &Url) -> String {
    let doc = scraper::Html::parse_document(html);
    if let Ok(title_sel) = scraper::Selector::parse("title") {
        if let Some(elem) = doc.select(&title_sel).next() {
            let t = elem.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if !t.is_empty() {
                // Return first chunk before | or -
                let short = t.split(&['|', '-'][..]).next().unwrap_or(&t).trim();
                if !short.is_empty() {
                    return short.to_string();
                }
            }
        }
    }

    url.host_str().unwrap_or("Career Page").to_string()
}

fn find_regex_match(text: &str, pattern_str: &str) -> Option<String> {
    let marker = if pattern_str.contains("greenhouse") {
        "greenhouse.io/"
    } else if pattern_str.contains("lever") {
        "jobs.lever.co/"
    } else if pattern_str.contains("ashby") {
        "jobs.ashbyhq.com/"
    } else {
        return None;
    };

    if let Some(idx) = text.find(marker) {
        let remainder = &text[idx + marker.len()..];
        if let Some(query) = remainder.strip_prefix("embed/job_board?for=") {
            let token: String = query
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if !token.is_empty() {
                return Some(token);
            }
        }

        let token: String = remainder
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if !token.is_empty() && token != "embed" {
            return Some(token);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_url_detection() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = HttpClient::new(5, 500);

        let res = rt.block_on(detect_source(
            &client,
            "https://boards.greenhouse.io/stripe",
        ));
        assert_eq!(res.kind, SourceKind::Greenhouse);
        assert_eq!(res.identifier, "stripe");

        let res = rt.block_on(detect_source(&client, "https://jobs.lever.co/figma"));
        assert_eq!(res.kind, SourceKind::Lever);
        assert_eq!(res.identifier, "figma");

        let res = rt.block_on(detect_source(&client, "https://jobs.ashbyhq.com/openai"));
        assert_eq!(res.kind, SourceKind::Ashby);
        assert_eq!(res.identifier, "openai");

        let res = rt.block_on(detect_source(
            &client,
            "https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite",
        ));
        assert_eq!(res.kind, SourceKind::Workday);
        assert_eq!(
            res.identifier,
            "https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite"
        );
        assert_eq!(res.name, "Nvidia");
    }
}
