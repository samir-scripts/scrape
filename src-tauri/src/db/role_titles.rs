use crate::error::{AppError, Result};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::collections::HashSet;

const ROLE_TITLE_SEED: &str =
    include_str!("../../../crates/scrape-core/data/occupations/seed_role_titles.tsv");
const ROLE_BATCH_SIZE: usize = 500;
const TERM_BATCH_SIZE: usize = 1_000;

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export)]
pub struct RoleTitleSuggestion {
    pub title: String,
    pub soc_code: String,
    pub priority_tier: i32,
}

#[derive(Debug)]
struct SeedTitle<'a> {
    soc_code: &'a str,
    title: &'a str,
    priority_tier: i64,
}

fn parse_seed(seed: &str) -> Result<Vec<SeedTitle<'_>>> {
    let mut lines = seed.lines();
    if lines.next() != Some("soc_code\ttitle\tpriority_tier") {
        return Err(AppError::Internal(
            "Role-title seed has an invalid header".into(),
        ));
    }
    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 {
            return Err(AppError::Internal(format!(
                "Role-title seed row {} must contain three tab-separated fields",
                index + 2
            )));
        }
        let (soc_code, title, tier) = (fields[0], fields[1], fields[2]);
        if soc_code.len() != 7
            || !soc_code.as_bytes()[..2].iter().all(u8::is_ascii_digit)
            || soc_code.as_bytes()[2] != b'-'
            || !soc_code.as_bytes()[3..].iter().all(u8::is_ascii_digit)
            || title.trim() != title
            || title.is_empty()
        {
            return Err(AppError::Internal(format!(
                "Role-title seed row {} has invalid values",
                index + 2
            )));
        }
        let priority_tier = tier.parse::<i64>().map_err(|_| {
            AppError::Internal(format!(
                "Role-title seed row {} has an invalid tier",
                index + 2
            ))
        })?;
        if !(1..=3).contains(&priority_tier) {
            return Err(AppError::Internal(format!(
                "Role-title seed row {} has an invalid tier",
                index + 2
            )));
        }
        rows.push(SeedTitle {
            soc_code,
            title,
            priority_tier,
        });
    }
    if rows.is_empty() {
        return Err(AppError::Internal("Role-title seed is empty".into()));
    }
    Ok(rows)
}

fn title_terms(title: &str) -> impl Iterator<Item = String> + '_ {
    title
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(str::to_lowercase)
}

pub async fn seed_role_titles(pool: &SqlitePool) -> Result<()> {
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM role_titles")
        .fetch_one(pool)
        .await?;
    if existing > 0 {
        return Ok(());
    }

    let rows = parse_seed(ROLE_TITLE_SEED)?;
    let mut tx = pool.begin().await?;
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM role_titles")
        .fetch_one(&mut *tx)
        .await?;
    if existing > 0 {
        tx.commit().await?;
        return Ok(());
    }

    for (batch_index, batch) in rows.chunks(ROLE_BATCH_SIZE).enumerate() {
        let first_id = batch_index * ROLE_BATCH_SIZE + 1;
        let mut query = QueryBuilder::<Sqlite>::new(
            "INSERT INTO role_titles (id, soc_code, title, priority_tier) ",
        );
        query.push_values(batch.iter().enumerate(), |mut row, (offset, title)| {
            row.push_bind((first_id + offset) as i64)
                .push_bind(title.soc_code)
                .push_bind(title.title)
                .push_bind(title.priority_tier);
        });
        query.build().execute(&mut *tx).await?;
    }

    let mut terms = Vec::new();
    for (index, title) in rows.iter().enumerate() {
        let id = (index + 1) as i64;
        terms.extend(title_terms(title.title).map(|term| (id, term)));
    }
    // Follow the term-first primary key to avoid random B-tree page writes.
    terms.sort_unstable_by(|left, right| left.1.cmp(&right.1).then(left.0.cmp(&right.0)));
    terms.dedup();
    for batch in terms.chunks(TERM_BATCH_SIZE) {
        let mut query =
            QueryBuilder::<Sqlite>::new("INSERT INTO role_title_terms (role_title_id, term) ");
        query.push_values(batch.iter(), |mut row, (id, term)| {
            row.push_bind(*id).push_bind(term);
        });
        query.build().execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn search_role_titles(
    pool: &SqlitePool,
    query: &str,
) -> Result<Vec<RoleTitleSuggestion>> {
    let query = query.trim();
    if query.len() > 256 {
        return Err(AppError::Internal(
            "Role-title search must be at most 256 UTF-8 bytes".into(),
        ));
    }
    let mut terms: Vec<String> = query
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(str::to_lowercase)
        .collect();
    let mut seen = HashSet::new();
    terms.retain(|term| seen.insert(term.clone()));

    if terms.len() > 16 {
        return Err(AppError::Internal(
            "Role-title search must contain at most 16 words".into(),
        ));
    }
    if terms.is_empty() {
        return Ok(sqlx::query_as(
            "SELECT title, soc_code, priority_tier FROM role_titles ORDER BY priority_tier, title COLLATE NOCASE, title LIMIT 50",
        )
        .fetch_all(pool)
        .await?);
    }

    let mut query_builder = QueryBuilder::<Sqlite>::new(
        "SELECT title, soc_code, priority_tier FROM role_titles WHERE id IN (",
    );
    for (index, term) in terms.iter().enumerate() {
        if index > 0 {
            query_builder.push(" INTERSECT ");
        }
        query_builder
            .push("SELECT role_title_id FROM role_title_terms WHERE term GLOB ")
            .push_bind(format!("{term}*"));
    }
    query_builder
        .push(") ORDER BY CASE WHEN title = ")
        .push_bind(query)
        .push(" COLLATE NOCASE THEN 0 WHEN substr(title, 1, length(")
        .push_bind(query)
        .push(")) = ")
        .push_bind(query)
        .push(" COLLATE NOCASE THEN 1 ELSE 2 END, priority_tier, title COLLATE NOCASE, title LIMIT 50");
    Ok(query_builder.build_query_as().fetch_all(pool).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_seed_rows_and_rejects_invalid_fields() {
        let valid = "soc_code\ttitle\tpriority_tier\n15-1252\tSoftware Engineer\t1\n";
        assert_eq!(parse_seed(valid).unwrap().len(), 1);
        assert!(parse_seed("soc_code\ttitle\tpriority_tier\n15-1252\tEngineer\t4\n").is_err());
        assert!(
            parse_seed("soc_code\ttitle\tpriority_tier\n15-1252\tEngineer\t1\textra\n").is_err()
        );
    }

    #[test]
    fn title_search_tokens_are_lowercase_unicode_words() {
        assert_eq!(
            title_terms("Senior Software Engineer / R&D").collect::<Vec<_>>(),
            ["senior", "software", "engineer", "r", "d"]
        );
    }
}
