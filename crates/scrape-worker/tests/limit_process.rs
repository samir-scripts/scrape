//! Actual worker processes against isolated HTTP fixtures; no public providers.
use scrape_core::{protocol::SourceScanStats, WorkerEvent, PROTOCOL_VERSION};
use serde_json::{json, Value};
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

struct Server {
    origin: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(handler: impl Fn(&str) -> String + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stopping = stop.clone();
        let seen = requests.clone();
        let thread = thread::spawn(move || {
            while !stopping.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(3)))
                            .unwrap();
                        let mut buf = [0; 4096];
                        let n = stream.read(&mut buf).unwrap();
                        let request = String::from_utf8_lossy(&buf[..n]);
                        let path = request.split_whitespace().nth(1).unwrap().to_string();
                        seen.lock().unwrap().push(path.clone());
                        let body = if path.ends_with("/robots.txt") {
                            "User-agent: *\nAllow: /".into()
                        } else {
                            handler(&path)
                        };
                        let _ = write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        );
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        });
        Self {
            origin,
            requests,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn run(
    server: &Server,
    sources: Vec<Value>,
    filters: Value,
    blocked: Vec<String>,
    preview: bool,
) -> Vec<WorkerEvent> {
    run_with_limits(server, sources, filters, blocked, preview, json!({}))
}
fn run_with_limits(
    server: &Server,
    sources: Vec<Value>,
    filters: Value,
    blocked: Vec<String>,
    preview: bool,
    enrichment: Value,
) -> Vec<WorkerEvent> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scrape-worker"));
    command.args(["--stdin", "--replay-origin", &server.origin]);
    if preview {
        command.arg("--dry-run");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let config = json!({"version": PROTOCOL_VERSION, "sources": sources, "filters": filters, "blocked_postings": blocked, "run_started_at":"2026-10-16T00:00:00Z", "limits": {"per_host_delay_ms": 0, "request_timeout_s": 3, "enrichment":enrichment}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config).unwrap())
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
fn source(kind: &str, id: &str, limit: Option<u32>) -> Value {
    json!({"id": id, "kind": kind, "name": "Fixture", "identifier": if matches!(kind,"jsonld"|"selector") {"https://fixture.test/careers"} else {id}, "config": {"__workflow_spec": {"max_jobs_per_run": limit}, "follow_links": if kind == "jsonld" { Value::String("a.detail".into()) } else { Value::Null }}})
}
fn summary(events: &[WorkerEvent]) -> &[SourceScanStats] {
    match events.last().unwrap() {
        WorkerEvent::Done { stats } => &stats.sources,
        _ => panic!("missing Done"),
    }
}
fn jobs(events: &[WorkerEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .count()
}

#[test]
fn all_adapters_stop_at_their_default_or_configured_acceptance_limit() {
    for kind in ["lever", "ashby", "selector", "jsonld", "greenhouse"] {
        let server = Server::new(move |path| {
            if path.starts_with("/fixture.test/jobs/") {
                return r#"<img alt="Fixture Logo" src="/company.svg">"#.into();
            }
            if kind == "greenhouse" && !path.ends_with("?content=true") {
                let id: u64 = path
                    .split('/')
                    .next_back()
                    .unwrap()
                    .split('?')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                return json!({"id":id,"title":"Engineer","absolute_url":format!("https://fixture.test/jobs/{id}")}).to_string();
            }
            let records: Vec<Value> = (0..120).map(|id| match kind {
                "lever" => json!({"id":id.to_string(),"text":"Engineer","hostedUrl":format!("https://fixture.test/jobs/{id}")}),
                "ashby" => json!({"id":id.to_string(),"title":"Engineer","jobUrl":format!("https://fixture.test/jobs/{id}")}),
                "greenhouse" => json!({"id":id,"title":"Engineer","absolute_url":format!("https://fixture.test/jobs/{id}")}),
                _ => json!({"@type":"JobPosting","identifier":id.to_string(),"title":"Engineer","url":format!("/jobs/{id}")}),
            }).collect();
            match kind {
                "lever" => json!(records).to_string(),
                "ashby" | "greenhouse" => json!({"jobs":records}).to_string(),
                "jsonld" => format!("<script type=\"application/ld+json\">{}</script><a class=detail href=/unneeded>More</a>", json!(records)),
                _ => (0..120).map(|id| format!("<div class=job-item><a class=job-title href=/jobs/{id}>Engineer</a></div>")).collect(),
            }
        });
        let limit = if kind == "greenhouse" { Some(3) } else { None };
        let events = run(
            &server,
            vec![source(kind, "board", limit)],
            json!({}),
            vec![],
            false,
        );
        let count = limit.unwrap_or(20) as usize;
        assert_eq!(jobs(&events), count, "{kind}");
        assert_eq!(summary(&events)[0].examined, count as u64, "{kind}");
        assert!(summary(&events)[0].limit_reached);
        let requests = server.requests.lock().unwrap();
        if matches!(kind, "greenhouse" | "lever" | "ashby") {
            assert_eq!(
                requests
                    .iter()
                    .filter(|path| path.starts_with("/fixture.test/jobs/"))
                    .count(),
                1,
                "exactly one branding page per workflow, not per job"
            );
            assert!(
                events
                    .iter()
                    .filter_map(|event| match event {
                        WorkerEvent::Job {
                            company_icon_url, ..
                        } => Some(company_icon_url.as_deref()),
                        _ => None,
                    })
                    .all(|logo| logo == Some("https://fixture.test/company.svg")),
                "{kind} must emit the provider logo through the real worker process"
            );
        }
        assert!(!requests.iter().any(|path| path.contains("unneeded")));
        if kind == "greenhouse" {
            assert_eq!(
                requests.len(),
                6,
                "listing, exactly three details, one robots check and one branding page"
            );
        }
    }
}

#[test]
fn rejected_invalid_duplicate_and_blocked_postings_do_not_consume_the_budget() {
    let server = Server::new(|_| {
        let mut records = vec![
            json!({"id":"reject","text":"Sales","hostedUrl":"https://fixture.test/reject"}),
            json!({"id":"unsafe","text":"Engineer","hostedUrl":"javascript:bad"}),
            json!({"id":"blocked","text":"Engineer","hostedUrl":"https://fixture.test/blocked"}),
        ];
        records.push(records[2].clone());
        records.extend((0..100).map(|id| json!({"id":id.to_string(),"text":"Engineer","hostedUrl":format!("https://fixture.test/{id}")})));
        json!(records).to_string()
    });
    let block = scrape_core::PostingIdentity::for_posting(
        Some(scrape_core::SourceKind::Lever),
        Some("board"),
        Some("blocked"),
        "https://fixture.test/blocked",
    )
    .unwrap()
    .key;
    let events = run(
        &server,
        vec![source("lever", "board", Some(5))],
        json!({"include_keywords":["Engineer"]}),
        vec![block],
        false,
    );
    assert_eq!(jobs(&events), 5);
    let scan = &summary(&events)[0];
    assert_eq!(
        (
            scan.examined,
            scan.matched,
            scan.blocked,
            scan.duplicates,
            scan.invalid
        ),
        (9, 6, 1, 1, 1)
    );
}

#[test]
fn preview_is_capped_at_ten_or_the_lower_limit_and_global_budgets_are_independent() {
    let server = Server::new(|_| {
        json!((0..100).map(|id| json!({"id":id.to_string(),"text":"Engineer","hostedUrl":format!("https://fixture.test/{id}")})).collect::<Vec<_>>()).to_string()
    });
    for (limit, expected) in [(Some(5), 5), (Some(50), 10), (None, 10)] {
        let events = run(
            &server,
            vec![source("lever", "one", limit)],
            json!({}),
            vec![],
            true,
        );
        assert_eq!(jobs(&events), expected);
        assert!(summary(&events)[0].limit_reached);
    }
    let events = run(
        &server,
        vec![source("lever", "one", None), source("lever", "two", None)],
        json!({}),
        vec![],
        false,
    );
    assert_eq!(jobs(&events), 40);
    assert_eq!(summary(&events).len(), 2);
    assert!(summary(&events)
        .iter()
        .all(|s| s.limit == 20 && s.limit_reached));
}

#[test]
fn authored_large_board_explains_low_counts_without_changing_restrictions() {
    // Authored diagnostic control, not the missing original Cloudflare response/config.
    let server = Server::new(|_| {
        json!((0..438).map(|id| {
        let title = if id < 2 { "Software Engineer Intern" } else if id < 400 { "Software Engineer" } else { "Sales Manager" };
        json!({"id":id.to_string(),"text":title,"hostedUrl":format!("https://fixture.test/{id}"),"categories":{"location":"Remote","commitment": if id < 2 {"Internship"} else {"Full-time"}}})
    }).collect::<Vec<_>>()).to_string()
    });
    let configurations = [
        (json!({}), 438),
        (
            json!({"matching":{"keywords":{"any":["Engineer"],"fields":["title"]}}}),
            400,
        ),
        (
            json!({"include_keywords":["Software","Engineer","Intern"],"exclude_keywords":["MBA","Sales","Marketing"],"locations":["Remote","San Francisco, CA","Austin, TX"],"role_types":["internship"]}),
            2,
        ),
    ];
    for (filters, expected) in configurations {
        let events = run(
            &server,
            vec![source("lever", "board", Some(500))],
            filters,
            vec![],
            false,
        );
        assert_eq!(jobs(&events), expected);
        let scan = &summary(&events)[0];
        scan.validate().unwrap();
        assert_eq!(scan.examined, 438);
        assert_eq!(scan.accepted, expected as u64);
        assert_eq!(scan.rejected, 438 - expected as u64);
        assert_eq!(
            scan.rejections.iter().map(|r| r.count).sum::<u64>(),
            scan.rejected
        );
        assert_eq!(
            scan.examined,
            scan.accepted + scan.blocked + scan.rejected + scan.invalid + scan.duplicates
        );
        assert_eq!(
            scan.termination,
            scrape_core::protocol::CoverageTermination::Exhausted
        );
        assert_eq!(
            scan.provider_total, None,
            "Do not invent provider totals from array length"
        );
        if expected == 2 {
            assert!(scan.rejections.iter().any(|r| r.reason
                == scrape_core::matching::RejectionReason::LegacyRoleType
                && r.count == 398));
            assert!(scan
                .rejection_examples
                .iter()
                .all(|e| e.primary == scrape_core::matching::RejectionReason::LegacyRoleType));
        }
    }
}

fn greenhouse_listing(id: u64, title: &str) -> Value {
    json!({"id":id,"title":title,"absolute_url":format!("https://fixture.test/jobs/{id}"),"company_name":"Fixture","location":{"name":"Remote"},"content":"<p>Authoritative listing description.</p>"})
}
fn detail_requests(server: &Server) -> Vec<String> {
    server
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|path| path.contains("?pay_transparency=true"))
        .cloned()
        .collect()
}
#[test]
fn late_matches_reach_the_cap_without_enriching_known_rejections() {
    let server = Server::new(|path| {
        if path.ends_with("?content=true") {
            return json!({"jobs":(0..200).map(|id| greenhouse_listing(id,if id < 140 {"Sales"} else {"Engineer"})).collect::<Vec<_>>(),"meta":{"total":200}}).to_string();
        }
        if path.contains("?pay_transparency=true") {
            let id: u64 = path
                .rsplit('/')
                .next()
                .unwrap()
                .split('?')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            return json!({"id":id,"title":"Concurrent renamed Sales title","absolute_url":"https://changed.example.test/job","content":"Different detail description","first_published":"2026-10-15T12:00:00Z","pay_input_ranges":[{"min_cents":5025,"max_cents":6075,"currency_type":"USD","title":"Hourly compensation"}]}).to_string();
        }
        "<img alt='Fixture Logo' src='/employer.svg'>".into()
    });
    let events = run(
        &server,
        vec![source("greenhouse", "board", None)],
        json!({"matching":{"keywords":{"any":["Engineer"],"fields":["title"]},"posting_date":{"last_hours":72,"timezone":"UTC"},"compensation":{"min_minor":5000,"currency":"USD","interval":"hour"}}}),
        vec![],
        false,
    );
    assert_eq!(jobs(&events), 20);
    let scan = &summary(&events)[0];
    scan.validate().unwrap();
    assert_eq!(
        (
            scan.examined,
            scan.rejected,
            scan.accepted,
            scan.unresolved,
            scan.enrichment_warnings
        ),
        (160, 140, 20, 0, 0)
    );
    assert_eq!(scan.provider_total, Some(200));
    assert_eq!(
        scan.outcome,
        Some(scrape_core::protocol::ScanOutcome::Success)
    );
    assert_eq!(
        scan.termination,
        scrape_core::protocol::CoverageTermination::JobLimit
    );
    let details = detail_requests(&server);
    assert_eq!(details.len(), 20);
    assert!(details.iter().all(|path| {
        let id: u64 = path
            .rsplit('/')
            .next()
            .unwrap()
            .split('?')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        (140..160).contains(&id)
    }));
    for event in events {
        if let WorkerEvent::Job {
            title,
            url,
            description_text,
            date_posted,
            salary,
            ..
        } = event
        {
            assert_eq!(title, "Engineer");
            assert!(url.starts_with("https://fixture.test/jobs/"));
            assert_eq!(
                description_text.as_deref(),
                Some("Authoritative listing description.")
            );
            assert_eq!(date_posted.as_deref(), Some("2026-10-15T12:00:00Z"));
            assert_eq!(salary.unwrap().ranges[0].min_minor, Some(5025));
        }
    }
}
#[test]
fn optional_details_warn_without_failing_or_inventing_missing_fields() {
    let server = Server::new(|path| {
        if path.ends_with("?content=true") {
            json!({"jobs":[greenhouse_listing(1,"Engineer")]}).to_string()
        } else if path.contains("?pay_transparency=true") {
            "not-json".into()
        } else {
            "<html></html>".into()
        }
    });
    let events = run(
        &server,
        vec![source("greenhouse", "board", None)],
        json!({"matching":{"keywords":{"any":["Engineer"],"fields":["title"]}}}),
        vec![],
        false,
    );
    assert_eq!(jobs(&events), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, WorkerEvent::Error { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        WorkerEvent::Warning {
            eligibility_incomplete: false,
            ..
        }
    )));
    let scan = &summary(&events)[0];
    scan.validate().unwrap();
    assert_eq!(
        scan.outcome,
        Some(scrape_core::protocol::ScanOutcome::SuccessWithWarnings)
    );
    assert_eq!(
        (scan.enrichment_warnings, scan.invalid, scan.unresolved),
        (1, 0, 0)
    );
    assert!(events.iter().any(|event| matches!(
        event,
        WorkerEvent::Job {
            date_posted: None,
            salary: None,
            ..
        }
    )));
}
#[test]
fn unavailable_required_evidence_is_partial_and_never_broadens_a_negative_rule() {
    for filters in [
        json!({"query":"description NOT LIKE '%Senior%'"}),
        json!({"matching":{"posting_date":{"last_hours":72,"timezone":"UTC"}}}),
        json!({"matching":{"compensation":{"min_minor":5000,"interval":"hour","currency":"USD"}}}),
    ] {
        let server = Server::new(|path| {
            if path.ends_with("?content=true") {
                json!({"jobs":[{"id":1,"title":"Engineer","absolute_url":"https://fixture.test/jobs/1"}]}).to_string()
            } else {
                "not-json".into()
            }
        });
        let events = run(
            &server,
            vec![source("greenhouse", "board", None)],
            filters,
            vec![],
            false,
        );
        assert_eq!(jobs(&events), 0);
        let scan = &summary(&events)[0];
        scan.validate().unwrap();
        assert_eq!(
            (
                scan.examined,
                scan.rejected,
                scan.unresolved,
                scan.enrichment_warnings
            ),
            (1, 0, 1, 1)
        );
        assert_eq!(
            scan.outcome,
            Some(scrape_core::protocol::ScanOutcome::Partial)
        );
        assert!(events.iter().any(|event| matches!(
            event,
            WorkerEvent::Warning {
                eligibility_incomplete: true,
                ..
            }
        )));
    }
}
#[test]
fn missing_required_fields_in_a_valid_detail_are_partial_but_explicit_unknown_inclusion_can_match()
{
    let server = Server::new(|path| {
        if path.ends_with("?content=true") {
            json!({"jobs":[greenhouse_listing(1,"Engineer")]}).to_string()
        } else if path.contains("?pay_transparency=true") {
            greenhouse_listing(1, "Engineer").to_string()
        } else {
            "<html></html>".into()
        }
    });
    for include_unknown in [false, true] {
        let events = run(
            &server,
            vec![source("greenhouse", "board", None)],
            json!({"matching":{"posting_date":{"last_hours":72,"timezone":"UTC","include_unknown":include_unknown}}}),
            vec![],
            false,
        );
        assert_eq!(jobs(&events), usize::from(include_unknown));
        let scan = &summary(&events)[0];
        scan.validate().unwrap();
        assert_eq!(scan.unresolved, u64::from(!include_unknown));
        assert_eq!(
            scan.outcome,
            Some(if include_unknown {
                scrape_core::protocol::ScanOutcome::Success
            } else {
                scrape_core::protocol::ScanOutcome::Partial
            })
        );
    }
}
#[test]
fn unresolved_listing_fields_get_their_detail_opportunity() {
    let server = Server::new(|path| {
        if path.ends_with("?content=true") {
            json!({"jobs":[{"id":1,"title":"Sales","absolute_url":"https://fixture.test/jobs/1","company_name":"Fixture"}]}).to_string()
        } else if path.contains("?pay_transparency=true") {
            json!({"id":1,"title":"Renamed","absolute_url":"https://changed.test/1","content":"<p>Engineer</p>"}).to_string()
        } else {
            "<html></html>".into()
        }
    });
    let events = run(
        &server,
        vec![source("greenhouse", "board", None)],
        json!({"matching":{"keywords":{"any":["Engineer"],"fields":["title","description"]}}}),
        vec![],
        false,
    );
    assert_eq!(jobs(&events), 1);
    assert_eq!(detail_requests(&server).len(), 1);
    assert!(events.iter().any(|event|matches!(event,WorkerEvent::Job{title,description_text:Some(text),..} if title=="Sales"&&text=="Engineer")));
}
#[test]
fn request_bound_distinguishes_optional_warnings_from_incomplete_eligibility() {
    let server = Server::new(|path| {
        if path.ends_with("?content=true") {
            json!({"jobs":(0..4).map(|id|greenhouse_listing(id,"Engineer")).collect::<Vec<_>>(),"meta":{"total":4}}).to_string()
        } else {
            "<html></html>".into()
        }
    });
    for required in [false, true] {
        let filters = if required {
            json!({"matching":{"posting_date":{"last_hours":72,"timezone":"UTC"}}})
        } else {
            json!({})
        };
        let events = run_with_limits(
            &server,
            vec![source("greenhouse", "board", Some(3))],
            filters,
            vec![],
            false,
            json!({"max_requests":0}),
        );
        assert!(detail_requests(&server).is_empty());
        let scan = &summary(&events)[0];
        scan.validate().unwrap();
        assert_eq!(scan.enrichment_warnings, 1);
        assert_eq!(
            scan.outcome,
            Some(if required {
                scrape_core::protocol::ScanOutcome::Partial
            } else {
                scrape_core::protocol::ScanOutcome::SuccessWithWarnings
            })
        );
        assert_eq!(
            scan.termination,
            if required {
                scrape_core::protocol::CoverageTermination::RequestLimit
            } else {
                scrape_core::protocol::CoverageTermination::JobLimit
            }
        );
        assert_eq!(scan.unresolved, if required { 4 } else { 0 });
    }
}
#[test]
fn detail_deadline_stops_scheduling_additional_requests() {
    let server = Server::new(|path| {
        if path.ends_with("?content=true") {
            json!({"jobs":(0..3).map(|id|greenhouse_listing(id,"Engineer")).collect::<Vec<_>>()} )
                .to_string()
        } else if path.contains("?pay_transparency=true") {
            std::thread::sleep(Duration::from_millis(1000));
            greenhouse_listing(0, "Engineer").to_string()
        } else {
            "<html></html>".into()
        }
    });
    let events = run_with_limits(
        &server,
        vec![source("greenhouse", "board", None)],
        json!({}),
        vec![],
        false,
        json!({"timeout_ms":500}),
    );
    assert_eq!(jobs(&events), 3);
    assert_eq!(detail_requests(&server).len(), 1);
    let scan = &summary(&events)[0];
    scan.validate().unwrap();
    assert_eq!(
        scan.termination,
        scrape_core::protocol::CoverageTermination::TimeLimit
    );
    assert_eq!(scan.enrichment_warnings, 1);
    assert_eq!(
        scan.outcome,
        Some(scrape_core::protocol::ScanOutcome::SuccessWithWarnings)
    );
}

#[test]
fn duplicate_candidates_do_not_spend_the_remaining_detail_allowance() {
    let server = Server::new(|path| {
        if path.ends_with("?content=true") {
            let mut jobs = vec![greenhouse_listing(1, "Engineer"); 110];
            jobs.push(greenhouse_listing(2, "Engineer"));
            return json!({"jobs":jobs}).to_string();
        }
        if path.contains("?pay_transparency=true") {
            let id: u64 = path
                .rsplit('/')
                .next()
                .unwrap()
                .split('?')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            let mut job = greenhouse_listing(id, "Engineer");
            job["first_published"] = json!("2026-10-15T00:00:00Z");
            return job.to_string();
        }
        "<html></html>".into()
    });
    let events = run_with_limits(
        &server,
        vec![source("greenhouse", "board", Some(2))],
        json!({"matching":{"posting_date":{"last_hours":72,"timezone":"UTC"}}}),
        vec![],
        false,
        json!({"max_requests":2}),
    );
    assert_eq!(jobs(&events), 2);
    assert_eq!(detail_requests(&server).len(), 2);
    let scan = &summary(&events)[0];
    scan.validate().unwrap();
    assert_eq!(
        (scan.duplicates, scan.unresolved, scan.enrichment_warnings),
        (109, 0, 0)
    );
}

#[test]
fn malformed_records_are_counted_and_isolated_but_a_missing_listing_collection_fails() {
    let server = Server::new(|path| {
        if path.ends_with("?content=true") {
            json!({"jobs":[{"id":0,"absolute_url":"https://fixture.test/missing-title"},greenhouse_listing(1,"Engineer")]}).to_string()
        } else if path.contains("?pay_transparency=true") {
            greenhouse_listing(1, "Engineer").to_string()
        } else {
            "<html></html>".into()
        }
    });
    let events = run(
        &server,
        vec![source("greenhouse", "board", None)],
        json!({}),
        vec![],
        false,
    );
    assert_eq!(jobs(&events), 1);
    let scan = &summary(&events)[0];
    scan.validate().unwrap();
    assert_eq!((scan.examined, scan.invalid, scan.accepted), (2, 1, 1));
    assert_eq!(
        scan.outcome,
        Some(scrape_core::protocol::ScanOutcome::Partial)
    );
    assert_eq!(detail_requests(&server).len(), 1);
    let malformed = Server::new(|_| json!({"error":"missing jobs collection"}).to_string());
    let events = run(
        &malformed,
        vec![source("greenhouse", "board", None)],
        json!({}),
        vec![],
        false,
    );
    assert_eq!(jobs(&events), 0);
    assert_eq!(
        summary(&events)[0].outcome,
        Some(scrape_core::protocol::ScanOutcome::Failed)
    );
    assert!(events.iter().any(|event|matches!(event,WorkerEvent::Error{message,..} if message.contains("missing field `jobs`"))));
}
