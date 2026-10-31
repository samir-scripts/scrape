//! End-to-end public discovery through the worker process and explicit replay transport.
use scrape_core::{WorkerEvent, PROTOCOL_VERSION};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

struct Fixture {
    origin: String,
    paths: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Fixture {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let paths = Arc::new(Mutex::new(Vec::new()));
        let captured = paths.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let thread = thread::spawn(move || {
            while !stopping.load(Ordering::SeqCst) {
                let (mut socket, _) = match listener.accept() {
                    Ok(value) => value,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("{error}"),
                };
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut input = Vec::new();
                let mut chunk = [0; 8192];
                loop {
                    let n = match socket.read(&mut chunk) {
                        Ok(n) => n,
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(error) => panic!("Fixture request read failed: {error}"),
                    };
                    if n == 0 {
                        break;
                    }
                    input.extend_from_slice(&chunk[..n]);
                    if input.windows(4).any(|part| part == b"\r\n\r\n") || input.len() >= 65536 {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&input);
                let Some(path) = request.split_whitespace().nth(1) else {
                    continue;
                };
                captured.lock().unwrap().push(path.to_owned());
                let (status, content_type, extra, body) = response(path);
                let _ = write!(socket, "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        Self {
            origin,
            paths,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn posting(title: &str, company: &str, date: &str, country: &str) -> String {
    format!(
        r#"<script type="application/ld+json">{{"@context":"https://schema.org","@type":"JobPosting","title":"{title}","hiringOrganization":{{"name":"{company}"}},"datePosted":"{date}","jobLocation":{{"@type":"Place","address":{{"addressCountry":"{country}","addressRegion":"CA"}}}},"description":"Build reliable distributed software systems."}}</script>"#
    )
}

fn evidence_posting(
    case: &str,
    identifier: bool,
    date: Option<&str>,
    url: Option<&str>,
) -> serde_json::Value {
    let mut value = json!({
        "@type":"JobPosting", "title":format!("Software Engineer {case}"),
        "hiringOrganization":{"name":"Actual Employer"},
        "jobLocation":{"address":{"addressCountry":"US"}},
        "description":"Responsibilities include building production software systems. Qualifications include professional engineering experience."
    });
    if identifier {
        value["identifier"] = json!({"value":format!("req-{case}")});
    }
    if let Some(date) = date {
        value["datePosted"] = json!(date);
    }
    if let Some(url) = url {
        value["url"] = json!(url);
    }
    value
}

fn evidence_html(value: &serde_json::Value) -> String {
    format!("<script type='application/ld+json'>{value}</script>")
}

fn publication_response(path: &str) -> Option<String> {
    if let Some(case) = path.strip_prefix("/evidence.example/careers/") {
        if !matches!(
            case,
            "listed-id"
                | "plain-listed-id"
                | "fallback-title"
                | "mismatch-id"
                | "missing-detail-id"
                | "mismatch-title"
                | "listing-conflict"
                | "detail-conflict"
                | "relative-listing"
                | "redirected"
        ) {
            return None;
        }
        let target = format!("https://evidence.example/jobs/{case}");
        let has_id = !matches!(case, "fallback-title" | "mismatch-title");
        let date = (case != "relative-listing").then_some("2026-10-30T09:00:00Z");
        let mut value = evidence_posting(case, has_id, date, Some(&target));
        value["hiringOrganization"]["name"] = json!("Untrusted Listing Employer");
        value["jobLocation"]["address"]["addressCountry"] = json!("GB");
        if case == "listing-conflict" {
            let mut conflicting = value.clone();
            conflicting["datePosted"] = json!("2026-10-28T09:00:00Z");
            return Some(evidence_html(&json!([value, conflicting])));
        }
        return Some(format!(
            "{}{}",
            evidence_html(&value),
            if case == "relative-listing" {
                "<p>Posted: 1 day ago</p>"
            } else {
                ""
            }
        ));
    }
    let case = path.strip_prefix("/evidence.example/jobs/")?;
    if matches!(
        case,
        "plain-closed" | "plain-open-footer" | "plain-listed-id"
    ) {
        let identifier = if case == "plain-listed-id" {
            " data-job-id='req-plain-listed-id'"
        } else {
            ""
        };
        let date = if case == "plain-listed-id" {
            ""
        } else {
            "<p>Posted: October 30, 2026</p>"
        };
        let closure = if case == "plain-closed" {
            "<p>No longer accepting applications.</p>"
        } else {
            "<footer>A previous role has been filled.</footer>"
        };
        return Some(format!("<meta property='og:site_name' content='Actual Employer'><main><article{identifier}><h1>Software Engineer {case}</h1><p class='location'>Austin, Texas, United States</p>{date}{closure}<a href='mailto:jobs@evidence.example'>Apply by email</a><h2>Responsibilities</h2><p>Build production software systems.</p><h2>Qualifications</h2><p>Professional engineering experience and Python skills.</p></article></main>"));
    }
    let mut value = evidence_posting(
        case,
        !matches!(
            case,
            "fallback-title" | "mismatch-title" | "missing-detail-id"
        ),
        None,
        None,
    );
    let mut visible = String::new();
    match case {
        "listed-id" | "fallback-title" | "listing-conflict" | "missing-detail-id" | "relative-listing" | "redirect-target" => {}
        "mismatch-id" => value["identifier"]["value"] = json!("different-requisition"),
        "mismatch-title" => value["title"] = json!("Software Engineer Different Title"),
        "detail-conflict" => {
            value["datePosted"] = json!("2026-10-28T09:00:00Z");
            visible.push_str("<p>Posted: October 30, 2026</p>");
        }
        "updated-only" => visible.push_str("<p>Updated: October 30, 2026</p>"),
        "ambiguous-slash" => visible.push_str("<p>Posted: 10/30/2026</p>"),
        "contradictory" => {
            value["datePosted"] = json!("2026-10-30T09:00:00Z");
            visible.push_str("<p>Posted: October 28, 2026</p>");
        }
        "closed" => {
            value["datePosted"] = json!("2026-10-30T09:00:00Z");
            visible.push_str("<main><h1>Software Engineer closed</h1><p>This position has been filled.</p></main>");
        }
        "expired-timestamp" => {
            value["datePosted"] = json!("2026-10-30T09:00:00Z");
            value["validThrough"] = json!("2026-10-31T11:59:59Z");
        }
        "expired-date" => {
            value["datePosted"] = json!("2026-10-30T09:00:00Z");
            value["validThrough"] = json!("2026-10-30");
        }
        "future-expiry" => {
            value["datePosted"] = json!("2026-10-30T09:00:00Z");
            value["validThrough"] = json!("2026-10-31");
        }
        "definition-list" => visible.push_str("<main><h1>Software Engineer definition-list</h1><dl><dt>Posted:</dt><dd>Oct. 30, 2026</dd></dl></main>"),
        "narrow-visible" => {
            value["datePosted"] = json!("2026-10-30");
            visible.push_str("<p>Posted: <time datetime='2026-10-30T09:00:00Z'>October 30, 2026</time></p>");
        }
        "narrow-structured" => {
            value["datePosted"] = json!("2026-10-30T09:00:00Z");
            visible.push_str("<p>Posted: October 30, 2026</p>");
        }
        "equivalent-offset" => {
            value["datePosted"] = json!("2026-10-30T09:00:00Z");
            visible.push_str("<p>Posted: <time datetime='2026-10-30T05:00:00-04:00'>October 30</time></p>");
        }
        "same-date" => {
            value["datePosted"] = json!("2026-10-30");
            visible.push_str("<p>Posted: October 30, 2026</p>");
        }
        "open-footer" => {
            value["datePosted"] = json!("2026-10-30T09:00:00Z");
            visible.push_str("<main><h1>Software Engineer open-footer</h1><p>Applications welcome.</p><footer>A previous position is closed. <p>Posted: October 28, 2026</p></footer></main>");
        }
        _ => return None,
    }
    Some(format!("{}{visible}", evidence_html(&value)))
}
fn response(path: &str) -> (&'static str, &'static str, &'static str, String) {
    if path == "/evidence.example/jobs/redirected" {
        return (
            "302 Found",
            "text/html",
            "Location: https://evidence.example/jobs/redirect-target\r\n",
            String::new(),
        );
    }
    if let Some(html) = publication_response(path) {
        return ("200 OK", "text/html", "", html);
    }
    if path.ends_with("/robots.txt") {
        return (
            "200 OK",
            "text/plain",
            "",
            "User-agent: *\nDisallow: /jobs/denied\n".into(),
        );
    }
    if path == "/crowded.example/sitemap.xml" {
        return ("200 OK", "application/xml", "", r#"<urlset><url><loc>https://crowded.example/assets/role-policy.png</loc></url></urlset>"#.into());
    }
    if path.ends_with("/sitemap.xml") {
        return (
            "200 OK",
            "application/xml",
            "",
            "<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"></urlset>".into(),
        );
    }
    let html = match path {
        "/oversized.example/careers" => r#"<a href="/jobs/invalid">Software Engineer invalid</a><a href="/jobs/valid">Software Engineer valid</a>"#.into(),
        "/oversized.example/jobs/invalid" => {
            let locations: Vec<_> = (0..65).map(|i| json!({"address":{"addressLocality":format!("City {i}"),"addressRegion":"CA","addressCountry":"US"}})).collect();
            let value = json!({"@type":"JobPosting","title":"Software Engineer invalid","identifier":"invalid","hiringOrganization":{"name":"Fiction Employer"},"datePosted":"2026-10-30T09:00:00Z","jobLocation":locations});
            evidence_html(&value)
        }
        "/oversized.example/jobs/valid" => posting("Software Engineer valid", "Fiction Employer", "2026-10-30T09:00:00Z", "US"),
        "/ncnonprofits.org/members/member-directory" => r#"<div class="member__title--list"><a href="/members/fiction">Fiction</a></div><div class="pager__item"><a href="?page=1">Next</a></div><a href="/jobs">Directory jobs</a><a href="/donate">Donate</a>"#.into(),
        "/ncnonprofits.org/members/member-directory?page=1" => r#"<div class="member__title--list"><a href="/members/fiction">Fiction</a></div>"#.into(),
        "/ncnonprofits.org/members/fiction" => r#"<div class="member__member-website"><a href="https://crowded.example/">https://crowded.example/</a></div><a href="/jobs">Directory jobs</a>"#.into(),
        "/crowded.example/" => r#"<a href="/careers">Careers</a>"#.into(),
        "/crowded.example/careers" => {
            let noise = (0..40).map(|n| format!(r#"<a href="/hiring-blog/{n}">Hiring</a><a href="/jobs/image{n}.png">Jobs</a>"#)).collect::<String>();
            format!(r#"{noise}<a href="/privacy">Jobs privacy</a><a href="/donate">Careers donation</a><a href="/company/partners">Our company</a><a href="/event-calendar/the-role-of-volunteers">Role of volunteers</a><a href="/jobs/backend-engineer">Backend Software Engineer</a><a href="/platform-engineer">Software Engineer Platform</a>"#)
        }
        "/crowded.example/jobs/backend-engineer" => r#"<html><head><meta property="og:site_name" content="Fiction Employer"></head><body><main><h1>Backend Software Engineer</h1><p class="location">Austin, Texas, United States</p><p>Posted: 2026-10-30</p><a href="mailto:jobs@crowded.example">Apply by email</a><section><h3>Responsibilities</h3><p>Build production software and support customers.</p></section><section><h3>Qualifications</h3><p>Professional engineering experience and Python skills.</p></section></main></body></html>"#.into(),
        "/crowded.example/platform-engineer" => posting("Software Engineer Platform", "Fiction Employer", "2026-10-30T09:00:00Z", "US"),
        "/employer.example/jobs/month-date" => format!("{}<p>Posted: October 30, 2026</p>", posting("Software Engineer", "Fiction Employer", "", "US").replace(r#","datePosted":"""#, "")),
        "/directory.example/companies" => r#"<a href="/companies/fiction">Fiction profile</a>"#.into(),
        "/directory.example/companies/fiction" => r#"<a href="https://employer.example/">Official website</a>"#.into(),
        "/employer.example/" => r#"<a href="/careers">Careers</a>"#.into(),
        "/employer.example/careers" => r#"<a href="/jobs/recent">Software Engineer</a><a href="/jobs/recent">Duplicate</a><a href="/jobs/old">Old job</a><a href="/jobs/unknown">Unknown job</a><a href="/jobs/updated">Updated job</a><a href="/jobs/foreign">Foreign job</a><a href="/jobs/conflicting">Conflicting dates</a><a href="/jobs/denied">Denied</a><a href="/jobs/private">Private redirect</a><a href="/careers">Cycle</a><a href="https://second.example/careers">More careers</a>"#.into(),
        "/employer.example/jobs/recent" => posting("Software Engineer", "Fiction Employer", "2026-10-15T00:00:00Z", "US"),
        "/employer.example/jobs/old" => posting("Old Software Engineer", "Fiction Employer", "2026-10-12T00:00:00Z", "US"),
        "/employer.example/jobs/unknown" => r#"<script type="application/ld+json">{"@type":"JobPosting","title":"Undated Software Engineer","hiringOrganization":{"name":"Fiction Employer"},"jobLocation":{"address":{"addressCountry":"US"}}}</script>"#.into(),
        "/employer.example/jobs/updated" => r#"<html><head><meta property="og:site_name" content="Fiction Employer"></head><body><main><h1>Updated Software Engineer</h1><p class="location">California, United States</p><p>Updated: <time datetime="2026-10-15T00:00:00Z">October 15, 2026</time></p><div class="job-description">Responsibilities include building resilient distributed software systems. Qualifications include professional software engineering experience and knowledge of software architecture and production services.</div></main></body></html>"#.into(),
        "/employer.example/jobs/foreign" => posting("Foreign Software Engineer", "Fiction Employer", "2026-10-15T00:00:00Z", "GB"),
        "/employer.example/jobs/conflicting" => format!("{}<p>Posted: <time datetime=\"2026-10-10T00:00:00Z\">October 10</time></p>", posting("Conflicting Software Engineer", "Fiction Employer", "2026-10-15T00:00:00Z", "US")),
        "/employer.example/jobs/relative" => format!("{}<p>Posted: 1 day ago</p>", posting("Relative Software Engineer", "Fiction Employer", "", "US").replace(r#","datePosted":"""#, "")),
        "/employer.example/jobs/private" => return ("302 Found", "text/html", "Location: http://127.0.0.1/private\r\n", String::new()),
        "/second.example/careers" => r#"<a href="/jobs/plain">Software Engineer</a>"#.into(),
        "/employer.example/careers/index" => r#"<html><head><meta property="og:site_name" content="Fiction Employer"></head><body><main><h2>Software Engineer</h2><p class="location">California, United States</p><p>Posted: <time datetime="2026-10-15T00:00:00Z">October 15, 2026</time></p><div class="job-description">Explore our engineering career opportunities and build reliable software systems.</div></main></body></html>"#.into(),
        "/employer.example/careers/email" => r#"<html><head><meta property="og:site_name" content="Fiction Employer"></head><body><main><h1>Backend Software Engineer</h1><a href="mailto:jobs@employer.example">Apply by email</a><section><h3>Responsibilities</h3><p>Build production software and support our customers.</p></section><section><h3>Qualifications</h3><p>Professional engineering experience and Python skills.</p></section></main></body></html>"#.into(),
        "/employer.example/careers/multiple" => r#"<html><head><meta property="og:site_name" content="Fiction Employer"></head><body><article data-job-id="alpha"><h2>Software Engineer Alpha</h2><p class="location">Austin, Texas, United States</p><p>Posted: <time datetime="2026-10-15T00:00:00Z">October 15</time></p><a href="/apply/alpha">Apply</a></article><article data-job-id="beta"><h2>Software Engineer Beta</h2><p class="location">Austin, Texas, United States</p><p>Posted: <time datetime="2026-10-15T00:00:00Z">October 15</time></p><a href="/apply/beta">Apply</a></article></body></html>"#.into(),
        "/second.example/jobs/plain" => r#"<html><head><meta property="og:site_name" content="Second Employer"></head><body><main><h1>Software Engineer</h1><p class="location">California, United States</p><p>Posted: <time datetime="2026-10-15T00:00:00Z">October 15, 2026</time></p><div class="job-description">Responsibilities include building resilient distributed software systems. Qualifications include professional software engineering experience and knowledge of software architecture and production services.</div></main></body></html>"#.into(),
        _ => return ("404 Not Found", "text/plain", "", "not found".into()),
    };
    ("200 OK", "text/html", "", html)
}
#[test]
fn directory_discovers_independent_employer_jobs_with_safe_bounds() {
    let fixture = Fixture::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &fixture.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = json!({"version":PROTOCOL_VERSION,"sources":[{"id":"directory","name":"Directory","kind":"web_discovery","identifier":"https://directory.example/companies","config":{"max_domains":4,"max_depth":5,"max_pages":100,"__cron_timezone":"UTC","__workflow_spec":{"max_jobs_per_run":20},"__source_id":"directory"}}],"filters":{"matching":{"target_roles":["Software Engineer"],"countries":["US"],"location_include_unknown":false,"posting_date":{"preset":"past_48h","timezone":"UTC","include_unknown":false}}},"max_jobs_per_run":20,"run_started_at":"2026-10-16T00:00:00Z","limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let jobs: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkerEvent::Job {
                title,
                company,
                url,
                date_posted,
                discovery_provenance,
                ..
            } => Some((
                title.as_str(),
                company.as_str(),
                url.as_str(),
                date_posted.as_deref(),
                discovery_provenance.as_ref(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(jobs.len(), 2, "{events:#?}");
    for (title, company, url, date, provenance) in jobs {
        assert_eq!(title, "Software Engineer");
        assert!(matches!(company, "Fiction Employer" | "Second Employer"));
        assert!(
            url.starts_with("https://employer.example/jobs/")
                || url.starts_with("https://second.example/jobs/")
        );
        assert_eq!(date, Some("2026-10-15T00:00:00Z"));
        let provenance = provenance.unwrap();
        assert_eq!(provenance.seed_url, "https://directory.example/companies");
        assert_eq!(provenance.final_url, url);
        assert_eq!(
            provenance.date_evidence.as_ref().unwrap().raw_value,
            "2026-10-15T00:00:00Z"
        );
    }
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::Warning { message, .. } if message.contains("Conflicting publication dates"))));
    let paths = fixture.paths.lock().unwrap();
    assert!(paths
        .iter()
        .any(|path| path == "/employer.example/jobs/conflicting"));
    assert_eq!(
        paths
            .iter()
            .filter(|path| path.as_str() == "/employer.example/jobs/recent")
            .count(),
        1
    );
    assert!(!paths
        .iter()
        .any(|path| path.contains("/jobs/denied") || path == "/127.0.0.1/private"));
}

#[test]
fn relative_publication_remains_observed_evidence_not_a_timestamp() {
    let fixture = Fixture::new();
    let started = chrono::Utc::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &fixture.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = json!({"version":PROTOCOL_VERSION,"sources":[{"id":"direct","name":"Directory","kind":"web_discovery","identifier":"https://employer.example/jobs/relative","config":{"max_domains":2,"max_depth":0,"max_pages":10}}],"filters":{"matching":{"posting_date":{"preset":"past_48h","timezone":"UTC","include_unknown":false}}},"max_jobs_per_run":20,"run_started_at":started.to_rfc3339(),"limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let evidence = events
        .iter()
        .find_map(|event| match event {
            WorkerEvent::Job {
                date_posted,
                discovery_provenance: Some(provenance),
                ..
            } => {
                assert!(date_posted.is_none());
                provenance.date_evidence.as_ref()
            }
            _ => None,
        })
        .expect("relative posting must be emitted");
    assert_eq!(evidence.raw_value, "1 day ago");
    assert_eq!(evidence.precision, "relative:86400:86400");
    assert!(
        chrono::DateTime::parse_from_rfc3339(&evidence.observed_at)
            .unwrap()
            .with_timezone(&chrono::Utc)
            >= started
    );
}

#[test]
fn shared_employer_page_keeps_distinct_requisition_identities() {
    let fixture = Fixture::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &fixture.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = json!({"version":PROTOCOL_VERSION,"sources":[{"id":"direct","name":"Directory","kind":"web_discovery","identifier":"https://employer.example/careers/multiple","config":{"max_domains":2,"max_depth":1,"max_pages":10}}],"filters":{"matching":{"target_roles":["Software Engineer"],"countries":["US"],"location_include_unknown":false,"posting_date":{"preset":"past_48h","timezone":"UTC","include_unknown":false}}},"max_jobs_per_run":20,"run_started_at":"2026-10-16T00:00:00Z","limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let mut jobs: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkerEvent::Job {
                external_id,
                title,
                company,
                url,
                ..
            } => Some((
                external_id.as_str(),
                title.as_str(),
                company.as_str(),
                url.as_str(),
            )),
            _ => None,
        })
        .collect();
    jobs.sort_unstable();
    assert_eq!(
        jobs,
        [
            (
                "alpha",
                "Software Engineer Alpha",
                "Fiction Employer",
                "https://employer.example/careers/multiple"
            ),
            (
                "beta",
                "Software Engineer Beta",
                "Fiction Employer",
                "https://employer.example/careers/multiple"
            ),
        ]
    );
}

#[test]
fn career_index_is_not_reported_as_a_job_detail() {
    let fixture = Fixture::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &fixture.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = json!({"version":PROTOCOL_VERSION,"sources":[{"id":"index","name":"Directory","kind":"web_discovery","identifier":"https://employer.example/careers/index","config":{"max_domains":2,"max_depth":1,"max_pages":10}}],"filters":{"matching":{"target_roles":["Software Engineer"],"countries":["US"],"location_include_unknown":false,"posting_date":{"preset":"past_48h","timezone":"UTC","include_unknown":false}}},"run_started_at":"2026-10-16T00:00:00Z","limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorkerEvent::Job { .. })),
        "{events:#?}"
    );
}

#[test]
fn direct_email_detail_is_examined_but_unknown_date_and_geography_do_not_match() {
    let fixture = Fixture::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &fixture.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = json!({"version":PROTOCOL_VERSION,"candidate_diagnostics":true,"sources":[{"id":"direct","name":"Directory","kind":"web_discovery","identifier":"https://employer.example/careers/email","config":{"max_domains":2,"max_depth":1,"max_pages":10}}],"filters":{"matching":{"target_roles":["Software Engineer"],"countries":["US"],"location_include_unknown":false,"posting_date":{"preset":"past_48h","timezone":"UTC","include_unknown":false}}},"run_started_at":"2026-10-16T00:00:00Z","limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::CandidateEvaluated { evaluation, .. }
        if evaluation.candidate.as_ref().is_some_and(|candidate| candidate.title == "Backend Software Engineer" && candidate.company == "Fiction Employer" && candidate.date_posted.is_none())
        && evaluation.unresolved.contains(&scrape_core::matching::RejectionReason::PostingDate)
        && evaluation.unresolved.contains(&scrape_core::matching::RejectionReason::Geography))), "{events:#?}");
    assert!(!events
        .iter()
        .any(|event| matches!(event, WorkerEvent::Job { .. })));
}

fn run_coverage_fixture(fixture: &Fixture, seed: &str) -> Vec<WorkerEvent> {
    run_publication_fixture(
        fixture,
        seed,
        "2026-10-31T12:00:00Z",
        Some(json!({"preset":"past_48h","timezone":"UTC","include_unknown":false})),
    )
}

fn run_publication_fixture(
    fixture: &Fixture,
    seed: &str,
    started: &str,
    posting_date: Option<serde_json::Value>,
) -> Vec<WorkerEvent> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &fixture.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut matching = json!({"target_roles":["Software Engineer"],"countries":["US"],"location_include_unknown":false});
    if let Some(posting_date) = posting_date {
        matching["posting_date"] = posting_date;
    }
    let input = json!({"version":PROTOCOL_VERSION,"sources":[{"id":"coverage","name":"Directory","kind":"web_discovery","identifier":seed,"config":{"max_domains":4,"max_depth":5,"max_pages":20}}],"filters":{"matching":matching},"max_jobs_per_run":20,"run_started_at":started,"limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn nonprofit_directory_reaches_direct_email_job_without_noise() {
    let fixture = Fixture::new();
    let events = run_coverage_fixture(
        &fixture,
        "https://ncnonprofits.org/members/member-directory",
    );
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::Job { url, .. } if url == "https://crowded.example/jobs/backend-engineer")), "{events:#?}");
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::Job { url, .. } if url == "https://crowded.example/platform-engineer")), "{events:#?}");
    let paths = fixture.paths.lock().unwrap();
    assert!(
        paths
            .iter()
            .any(|path| path == "/ncnonprofits.org/members/member-directory?page=1"),
        "{paths:#?}"
    );
    assert!(
        !paths.iter().any(|path| path.contains("hiring-blog")
            || path.ends_with(".png")
            || path.ends_with("/donate")
            || path == "/ncnonprofits.org/jobs"
            || path.ends_with("/company/partners")
            || path.contains("/event-calendar/")),
        "{paths:#?}"
    );
}

#[test]
fn structured_job_uses_attributable_visible_month_publication() {
    let fixture = Fixture::new();
    let events = run_coverage_fixture(&fixture, "https://employer.example/jobs/month-date");
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::Job { url, date_posted: Some(date), discovery_provenance: Some(provenance), .. }
        if url == "https://employer.example/jobs/month-date" && date == "2026-10-30"
            && provenance.date_evidence.as_ref().is_some_and(|evidence| evidence.raw_value == "October 30, 2026"))), "{events:#?}");
}

#[test]
fn listing_dates_require_exact_identity_and_preserve_the_listing_observation() {
    let fixture = Fixture::new();
    for case in ["listed-id", "plain-listed-id", "fallback-title"] {
        let events = run_coverage_fixture(
            &fixture,
            &format!("https://evidence.example/careers/{case}"),
        );
        let jobs: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, WorkerEvent::Job { .. }))
            .collect();
        assert_eq!(jobs.len(), 1, "{case}: {events:#?}");
        assert!(
            matches!(jobs[0], WorkerEvent::Job { company, date_posted: Some(date), discovery_provenance: Some(provenance), .. }
            if company == "Actual Employer" && date == "2026-10-30T09:00:00Z"
            && provenance.date_evidence.as_ref().is_some_and(|evidence|
                evidence.page_url == format!("https://evidence.example/careers/{case}")
                && evidence.locator == "JobPosting.datePosted"
                && evidence.raw_value == "2026-10-30T09:00:00Z")),
            "{events:#?}"
        );
    }
    for case in [
        "mismatch-id",
        "missing-detail-id",
        "mismatch-title",
        "listing-conflict",
        "detail-conflict",
        "redirected",
    ] {
        let events = run_coverage_fixture(
            &fixture,
            &format!("https://evidence.example/careers/{case}"),
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, WorkerEvent::Job { .. })),
            "{case}: {events:#?}"
        );
    }
    let paths = fixture.paths.lock().unwrap();
    for case in [
        "listed-id",
        "plain-listed-id",
        "fallback-title",
        "mismatch-id",
        "missing-detail-id",
        "mismatch-title",
        "listing-conflict",
        "detail-conflict",
        "redirected",
    ] {
        assert!(
            paths.contains(&format!("/evidence.example/jobs/{case}")),
            "{case}: {paths:#?}"
        );
    }
    assert!(
        paths.contains(&"/evidence.example/jobs/redirect-target".to_owned()),
        "{paths:#?}"
    );
}

#[test]
fn publication_controls_reject_updated_ambiguous_conflicting_closed_and_expired_roles() {
    let fixture = Fixture::new();
    for case in [
        "updated-only",
        "ambiguous-slash",
        "contradictory",
        "closed",
        "plain-closed",
        "expired-timestamp",
        "expired-date",
    ] {
        let events =
            run_coverage_fixture(&fixture, &format!("https://evidence.example/jobs/{case}"));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, WorkerEvent::Job { .. })),
            "{case}: {events:#?}"
        );
    }
    for case in [
        "definition-list",
        "narrow-visible",
        "narrow-structured",
        "equivalent-offset",
        "open-footer",
        "plain-open-footer",
        "future-expiry",
    ] {
        let events =
            run_coverage_fixture(&fixture, &format!("https://evidence.example/jobs/{case}"));
        let jobs: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, WorkerEvent::Job { .. }))
            .collect();
        assert_eq!(jobs.len(), 1, "{case}: {events:#?}");
        if case == "definition-list" {
            assert!(
                matches!(jobs[0], WorkerEvent::Job { date_posted: Some(date), discovery_provenance: Some(provenance), .. }
                if date == "2026-10-30" && provenance.date_evidence.as_ref().is_some_and(|evidence| evidence.raw_value == "Oct. 30, 2026" && evidence.locator == "dt + dd publication label")),
                "{events:#?}"
            );
        }
        if case == "narrow-visible" {
            assert!(
                matches!(jobs[0], WorkerEvent::Job { date_posted: Some(date), discovery_provenance: Some(provenance), .. }
                if date == "2026-10-30T09:00:00Z" && provenance.date_evidence.as_ref().is_some_and(|evidence| evidence.kind == "visible_publication")),
                "{events:#?}"
            );
        }
        if case == "narrow-structured" {
            assert!(
                matches!(jobs[0], WorkerEvent::Job { date_posted: Some(date), discovery_provenance: Some(provenance), .. }
                if date == "2026-10-30T09:00:00Z" && provenance.date_evidence.as_ref().is_some_and(|evidence| evidence.kind == "json_ld")),
                "{events:#?}"
            );
        }
    }
}

#[test]
fn unresolved_timezone_withholds_mixed_precision_and_date_only_expiry() {
    let fixture = Fixture::new();
    for case in ["narrow-visible", "future-expiry"] {
        let events = run_publication_fixture(
            &fixture,
            &format!("https://evidence.example/jobs/{case}"),
            "2026-10-31T12:00:00Z",
            None,
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, WorkerEvent::Job { .. })),
            "{case}: {events:#?}"
        );
        assert!(events.iter().any(|event| matches!(event, WorkerEvent::Warning { message, eligibility_incomplete: true, .. } if message.contains("timezone"))), "{events:#?}");
    }
    for case in ["same-date", "equivalent-offset"] {
        let events = run_publication_fixture(
            &fixture,
            &format!("https://evidence.example/jobs/{case}"),
            "2026-10-31T12:00:00Z",
            None,
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, WorkerEvent::Job { .. }))
                .count(),
            1,
            "{case}: {events:#?}"
        );
    }
}

#[test]
fn relative_listing_inheritance_keeps_the_original_page_and_observed_instant() {
    let fixture = Fixture::new();
    let started = chrono::Utc::now();
    let events = run_publication_fixture(
        &fixture,
        "https://evidence.example/careers/relative-listing",
        &started.to_rfc3339(),
        Some(json!({"preset":"past_48h","timezone":"UTC","include_unknown":false})),
    );
    let evidence = events
        .iter()
        .find_map(|event| match event {
            WorkerEvent::Job {
                date_posted,
                discovery_provenance: Some(provenance),
                ..
            } => {
                assert!(date_posted.is_none());
                provenance.date_evidence.as_ref()
            }
            _ => None,
        })
        .expect("relative listing must qualify through its fetched exact-ID detail");
    assert_eq!(
        evidence.page_url,
        "https://evidence.example/careers/relative-listing"
    );
    assert_eq!(evidence.raw_value, "1 day ago");
    assert_eq!(evidence.precision, "relative:86400:86400");
    let observed = chrono::DateTime::parse_from_rfc3339(&evidence.observed_at)
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert!(observed >= started);
    assert!(observed <= chrono::Utc::now());
}

#[test]
fn oversized_geography_is_reported_invalid_without_aborting_other_jobs() {
    let fixture = Fixture::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &fixture.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = json!({"version":PROTOCOL_VERSION,"candidate_diagnostics":true,"sources":[{"id":"coverage","name":"Directory","kind":"web_discovery","identifier":"https://oversized.example/careers","config":{"max_domains":2,"max_depth":2,"max_pages":10}}],"filters":{"matching":{"target_roles":["Software Engineer"],"countries":["US"],"posting_date":{"preset":"past_48h","timezone":"UTC","include_unknown":false}}},"run_started_at":"2026-10-31T12:00:00Z","limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::CandidateEvaluated { evaluation, .. } if evaluation.disposition == scrape_core::protocol::CandidateDisposition::Invalid && evaluation.candidate.is_none())), "{events:#?}");
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::Job { title, .. } if title == "Software Engineer valid")), "{events:#?}");
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::Done { stats } if stats.sources.iter().any(|source| source.invalid == 1))), "{events:#?}");
}

#[test]
fn primary_seed_precedes_same_origin_alternates_with_one_page_budget() {
    let fixture = Fixture::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &fixture.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = json!({"version":PROTOCOL_VERSION,"sources":[{"id":"seed","name":"Seed","kind":"web_discovery","identifier":"https://seed.example/primary","config":{"seed_urls":["https://seed.example/z"],"max_domains":1,"max_depth":1,"max_pages":1}}],"run_started_at":"2026-10-31T12:00:00Z","limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let paths = fixture.paths.lock().unwrap();
    assert!(
        paths.iter().any(|path| path == "/seed.example/primary"),
        "{paths:?}"
    );
    assert!(
        !paths.iter().any(|path| path == "/seed.example/z"),
        "{paths:?}"
    );
}
