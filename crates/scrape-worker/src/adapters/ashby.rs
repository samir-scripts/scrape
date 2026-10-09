use crate::client::{ClientError, HttpClient};
use scrape_core::{RoleType, WorkerEvent, WorkerSource};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AshbyResponse {
    #[serde(default)]
    jobs: Vec<AshbyJob>,
}

#[derive(Debug, Deserialize)]
struct AshbyJob {
    id: String,
    title: String,
    #[serde(rename = "jobUrl")]
    job_url: String,
    location: Option<String>,
    #[serde(rename = "employmentType")]
    employment_type: Option<String>,
    #[serde(rename = "publishedAt")]
    published_at: Option<String>,
}

pub async fn scrape_ashby(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let url = format!(
        "https://api.ashbyhq.com/posting-api/job-board/{}",
        source.identifier.trim()
    );

    let resp: AshbyResponse = client.fetch_json(&url).await?;
    let mut events = Vec::with_capacity(resp.jobs.len());

    for job in resp.jobs {
        let role_type = RoleType::infer(&job.title, job.employment_type.as_deref());
        let location = job.location.unwrap_or_default();

        events.push(WorkerEvent::Job {
            source_id: source.id.clone(),
            external_id: job.id,
            title: job.title,
            company: source.name.clone(),
            role_type,
            location,
            url: job.job_url,
            date_posted: job.published_at,
        });
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ashby_json() {
        let json = r#"{
            "jobs": [
                {
                    "id": "ashby-99",
                    "title": "Systems Architect",
                    "jobUrl": "https://jobs.ashbyhq.com/example/ashby-99",
                    "location": "New York, NY",
                    "employmentType": "Contract",
                    "publishedAt": "2026-10-02T10:00:00Z"
                }
            ]
        }"#;

        let resp: AshbyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.jobs.len(), 1);
        let job = &resp.jobs[0];
        assert_eq!(job.id, "ashby-99");
        let role = RoleType::infer(&job.title, job.employment_type.as_deref());
        assert_eq!(role, RoleType::Contract);
    }
}
