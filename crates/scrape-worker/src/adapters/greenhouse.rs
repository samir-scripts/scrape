use crate::client::{ClientError, HttpClient};
use scrape_core::{RoleType, WorkerEvent, WorkerSource};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GreenhouseResponse {
    #[serde(default)]
    jobs: Vec<GreenhouseJob>,
}

#[derive(Debug, Deserialize)]
struct GreenhouseJob {
    id: i64,
    title: String,
    absolute_url: String,
    location: Option<GreenhouseLocation>,
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GreenhouseLocation {
    name: Option<String>,
}

pub async fn scrape_greenhouse(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let url = format!(
        "https://boards-api.greenhouse.io/v1/boards/{}/jobs",
        source.identifier.trim()
    );

    let resp: GreenhouseResponse = client.fetch_json(&url).await?;
    let mut events = Vec::with_capacity(resp.jobs.len());

    for job in resp.jobs {
        let role_type = RoleType::infer(&job.title, None);
        let location = job.location.and_then(|l| l.name).unwrap_or_default();

        events.push(WorkerEvent::Job {
            source_id: source.id.clone(),
            external_id: job.id.to_string(),
            title: job.title,
            company: source.name.clone(),
            role_type,
            location,
            url: job.absolute_url,
            date_posted: job.updated_at,
        });
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_greenhouse_json() {
        let json = r#"{
            "jobs": [
                {
                    "id": 5012345,
                    "title": "Software Engineer, Data Intern",
                    "absolute_url": "https://boards.greenhouse.io/stripe/jobs/5012345",
                    "location": { "name": "Remote" },
                    "updated_at": "2026-10-01T12:00:00Z"
                }
            ]
        }"#;

        let resp: GreenhouseResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.jobs.len(), 1);
        let job = &resp.jobs[0];
        assert_eq!(job.id, 5012345);
        assert_eq!(job.title, "Software Engineer, Data Intern");
        let role_type = RoleType::infer(&job.title, None);
        assert_eq!(role_type, RoleType::Internship);
    }
}
