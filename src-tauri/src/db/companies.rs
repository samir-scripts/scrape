use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use ts_rs::TS;

use std::sync::LazyLock;

type TemplateCatalog = crate::services::template_catalog::WorkflowTemplateCatalog;
static TEMPLATE_CATALOG: LazyLock<std::result::Result<TemplateCatalog, String>> =
    LazyLock::new(|| {
        crate::services::template_catalog::catalog().map_err(|error| error.to_string())
    });

fn template_catalog() -> Result<&'static TemplateCatalog> {
    TEMPLATE_CATALOG
        .as_ref()
        .map_err(|error| AppError::Internal(error.clone()))
}

const COMPANY_RESULT_LIMIT: i64 = 250;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct CompanySuggestion {
    pub name: String,
    pub catalog_company_id: Option<String>,
    #[ts(type = "number")]
    pub jobs_seen: i64,
}

fn tokens(query: &str) -> Vec<String> {
    query
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn rank(name: &str, query: &str) -> u8 {
    if name.eq_ignore_ascii_case(query) {
        0
    } else if name
        .get(..query.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(query))
    {
        1
    } else {
        2
    }
}

pub async fn search_companies(pool: &SqlitePool, query: &str) -> Result<Vec<CompanySuggestion>> {
    let query = query.trim();
    if query.len() > 128 {
        return Err(AppError::Validation(
            "Company search must be at most 128 UTF-8 bytes".into(),
        ));
    }
    let search_terms = tokens(query);
    if search_terms.len() > 8 {
        return Err(AppError::Validation(
            "Company search must contain at most 8 words".into(),
        ));
    }
    let catalog = template_catalog()?;
    let mut suggestions: Vec<CompanySuggestion> = Vec::new();
    for preset in &catalog.companies {
        let template = catalog
            .templates
            .iter()
            .find(|template| template.id == preset.template_id)
            .ok_or_else(|| AppError::Internal("Company preset has no template".into()))?;
        let source = template
            .spec
            .sources
            .iter()
            .find(|source| source.id == preset.source_id)
            .ok_or_else(|| AppError::Internal("Company preset has no source".into()))?;
        let searchable = format!(
            "{} {} {:?} {} {:?}",
            preset.name,
            template.name,
            source.extractor.kind,
            source.extractor.url,
            template.readiness
        )
        .to_lowercase();
        if !search_terms.iter().all(|term| {
            searchable
                .split(|c: char| !c.is_alphanumeric())
                .any(|word| word.starts_with(term))
        }) {
            continue;
        }
        suggestions.push(CompanySuggestion {
            name: preset.name.clone(),
            catalog_company_id: Some(preset.id.clone()),
            jobs_seen: 0,
        });
    }

    if !query.is_empty() {
        let mut pattern = String::with_capacity(query.len() + 1);
        for character in query.chars() {
            if matches!(character, '\\' | '%' | '_') {
                pattern.push('\\');
            }
            pattern.push(character);
        }
        pattern.push('%');
        let local: Vec<(String, i64)> = sqlx::query_as(
            r#"SELECT MIN(company), COUNT(*) FROM jobs
               WHERE company <> '' AND company LIKE ? ESCAPE '\' COLLATE NOCASE
               GROUP BY company COLLATE NOCASE
               ORDER BY CASE WHEN MIN(company) = ? COLLATE NOCASE THEN 0 ELSE 1 END,
                        COUNT(*) DESC, MIN(company) COLLATE NOCASE, MIN(company)
               LIMIT ?"#,
        )
        .bind(pattern)
        .bind(query)
        .bind(COMPANY_RESULT_LIMIT)
        .fetch_all(pool)
        .await?;
        for (name, count) in local {
            if let Some(existing) = suggestions
                .iter_mut()
                .find(|item| item.name.eq_ignore_ascii_case(&name))
            {
                existing.jobs_seen = count;
            } else {
                suggestions.push(CompanySuggestion {
                    name,
                    catalog_company_id: None,
                    jobs_seen: count,
                });
            }
        }
    }

    suggestions.sort_by(|left, right| {
        rank(&left.name, query)
            .cmp(&rank(&right.name, query))
            .then_with(|| right.jobs_seen.cmp(&left.jobs_seen))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
    });
    suggestions.truncate(COMPANY_RESULT_LIMIT as usize);
    Ok(suggestions)
}

#[cfg(test)]
mod tests {
    #[test]
    fn search_tokenizer_splits_punctuation_without_rewriting_filter_values() {
        assert_eq!(
            super::tokens("Dollar-General, Inc."),
            ["dollar", "general", "inc"]
        );
        assert_eq!(super::rank("Dollar General", "Dollar General"), 0);
        assert_eq!(super::rank("Dollar General Logistics", "Dollar"), 1);
    }
}
