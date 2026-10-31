//! End-to-end public discovery through the worker process and explicit replay transport.
use scrape_core::{WorkerEvent, PROTOCOL_VERSION};
use serde_json::json;
use std::{io::{Read, Write}, net::TcpListener, process::{Command, Stdio}, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, thread, time::Duration};

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
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => { thread::sleep(Duration::from_millis(2)); continue; }
                    Err(error) => panic!("{error}"),
                };
                socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut input = [0; 8192];
                let Ok(n) = socket.read(&mut input) else { continue };
                let request = String::from_utf8_lossy(&input[..n]);
                let Some(path) = request.split_whitespace().nth(1) else { continue };
                captured.lock().unwrap().push(path.to_owned());
                let (status, content_type, extra, body) = response(path);
                let _ = write!(socket, "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        Self { origin, paths, stop, thread: Some(thread) }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) { self.stop.store(true, Ordering::SeqCst); self.thread.take().unwrap().join().unwrap(); }
}
fn posting(title: &str, company: &str, date: &str, country: &str) -> String {
    format!(r#"<script type="application/ld+json">{{"@context":"https://schema.org","@type":"JobPosting","title":"{title}","hiringOrganization":{{"name":"{company}"}},"datePosted":"{date}","jobLocation":{{"@type":"Place","address":{{"addressCountry":"{country}","addressRegion":"CA"}}}},"description":"Build reliable distributed software systems."}}</script>"#)
}
fn response(path: &str) -> (&'static str, &'static str, &'static str, String) {
    if path.ends_with("/robots.txt") { return ("200 OK", "text/plain", "", "User-agent: *\nDisallow: /jobs/denied\n".into()); }
    if path.ends_with("/sitemap.xml") { return ("200 OK", "application/xml", "", "<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"></urlset>".into()); }
    let html = match path {
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
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker")).args(["--stdin", "--replay-origin", &fixture.origin]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let input = json!({"version":PROTOCOL_VERSION,"sources":[{"id":"directory","name":"Directory","kind":"web_discovery","identifier":"https://directory.example/companies","config":{"max_domains":4,"max_depth":5,"max_pages":100,"__cron_timezone":"UTC","__workflow_spec":{"max_jobs_per_run":20},"__source_id":"directory"}}],"filters":{"matching":{"target_roles":["Software Engineer"],"countries":["US"],"location_include_unknown":false,"posting_date":{"preset":"past_48h","timezone":"UTC","include_unknown":false}}},"max_jobs_per_run":20,"run_started_at":"2026-10-16T00:00:00Z","limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child.stdin.take().unwrap().write_all(&serde_json::to_vec(&input).unwrap()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    let jobs: Vec<_> = events.iter().filter_map(|event| match event { WorkerEvent::Job { title, company, url, date_posted, discovery_provenance, .. } => Some((title.as_str(), company.as_str(), url.as_str(), date_posted.as_deref(), discovery_provenance.as_ref())), _ => None }).collect();
    assert_eq!(jobs.len(), 2, "{events:#?}");
    for (title, company, url, date, provenance) in jobs {
        assert_eq!(title, "Software Engineer");
        assert!(matches!(company, "Fiction Employer" | "Second Employer"));
        assert!(url.starts_with("https://employer.example/jobs/") || url.starts_with("https://second.example/jobs/"));
        assert_eq!(date, Some("2026-10-15T00:00:00Z"));
        let provenance = provenance.unwrap();
        assert_eq!(provenance.seed_url, "https://directory.example/companies");
        assert_eq!(provenance.final_url, url);
        assert_eq!(provenance.date_evidence.as_ref().unwrap().raw_value, "2026-10-15T00:00:00Z");
    }
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::Warning { message, .. } if message.contains("Conflicting publication dates"))));
    let paths = fixture.paths.lock().unwrap();
    assert!(paths.iter().any(|path| path == "/employer.example/jobs/conflicting"));
    assert_eq!(paths.iter().filter(|path| path.as_str() == "/employer.example/jobs/recent").count(), 1);
    assert!(!paths.iter().any(|path| path.contains("/jobs/denied") || path == "/127.0.0.1/private"));
}

#[test]
fn shared_employer_page_keeps_distinct_requisition_identities() {
    let fixture = Fixture::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &fixture.origin])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().unwrap();
    let input = json!({"version":PROTOCOL_VERSION,"sources":[{"id":"direct","name":"Directory","kind":"web_discovery","identifier":"https://employer.example/careers/multiple","config":{"max_domains":2,"max_depth":1,"max_pages":10}}],"filters":{"matching":{"target_roles":["Software Engineer"],"countries":["US"],"location_include_unknown":false,"posting_date":{"preset":"past_48h","timezone":"UTC","include_unknown":false}}},"max_jobs_per_run":20,"run_started_at":"2026-10-16T00:00:00Z","limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child.stdin.take().unwrap().write_all(&serde_json::to_vec(&input).unwrap()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    let mut jobs: Vec<_> = events.iter().filter_map(|event| match event {
        WorkerEvent::Job { external_id, title, company, url, .. } => Some((external_id.as_str(), title.as_str(), company.as_str(), url.as_str())),
        _ => None,
    }).collect();
    jobs.sort_unstable();
    assert_eq!(jobs, [
        ("alpha", "Software Engineer Alpha", "Fiction Employer", "https://employer.example/careers/multiple"),
        ("beta", "Software Engineer Beta", "Fiction Employer", "https://employer.example/careers/multiple"),
    ]);
}

#[test]
fn career_index_is_not_reported_as_a_job_detail() {
    let fixture = Fixture::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &fixture.origin])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().unwrap();
    let input = json!({"version":PROTOCOL_VERSION,"sources":[{"id":"index","name":"Directory","kind":"web_discovery","identifier":"https://employer.example/careers/index","config":{"max_domains":2,"max_depth":1,"max_pages":10}}],"filters":{"matching":{"target_roles":["Software Engineer"],"countries":["US"],"location_include_unknown":false,"posting_date":{"preset":"past_48h","timezone":"UTC","include_unknown":false}}},"run_started_at":"2026-10-16T00:00:00Z","limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child.stdin.take().unwrap().write_all(&serde_json::to_vec(&input).unwrap()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert!(!events.iter().any(|event| matches!(event, WorkerEvent::Job { .. })), "{events:#?}");
}

#[test]
fn direct_email_detail_is_examined_but_unknown_date_and_geography_do_not_match() {
    let fixture = Fixture::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &fixture.origin])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().unwrap();
    let input = json!({"version":PROTOCOL_VERSION,"candidate_diagnostics":true,"sources":[{"id":"direct","name":"Directory","kind":"web_discovery","identifier":"https://employer.example/careers/email","config":{"max_domains":2,"max_depth":1,"max_pages":10}}],"filters":{"matching":{"target_roles":["Software Engineer"],"countries":["US"],"location_include_unknown":false,"posting_date":{"preset":"past_48h","timezone":"UTC","include_unknown":false}}},"run_started_at":"2026-10-16T00:00:00Z","limits":{"per_host_delay_ms":0,"request_timeout_s":5}});
    child.stdin.take().unwrap().write_all(&serde_json::to_vec(&input).unwrap()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::CandidateEvaluated { evaluation, .. }
        if evaluation.candidate.as_ref().is_some_and(|candidate| candidate.title == "Backend Software Engineer" && candidate.company == "Fiction Employer" && candidate.date_posted.is_none())
        && evaluation.unresolved.contains(&scrape_core::matching::RejectionReason::PostingDate)
        && evaluation.unresolved.contains(&scrape_core::matching::RejectionReason::Geography))), "{events:#?}");
    assert!(!events.iter().any(|event| matches!(event, WorkerEvent::Job { .. })));
}
