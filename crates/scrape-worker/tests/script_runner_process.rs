use scrape_core::WorkerEvent;
use serde_json::json;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

fn run_worker_with_config(config: serde_json::Value) -> (std::process::Output, Vec<WorkerEvent>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config).unwrap())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    let events: Vec<WorkerEvent> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    (output, events)
}

#[test]
fn script_multi_turn_continuation_streams_and_matches_candidates() {
    let script_code = r#"
const fs = require('fs');
const input = JSON.parse(fs.readFileSync(0, 'utf-8'));

if (!input.continuation) {
    // Turn 1
    console.log(JSON.stringify({
        type: 'diagnostic',
        version: 2,
        level: 'info',
        message: 'Started turn 1 crawl'
    }));
    console.log(JSON.stringify({
        type: 'candidate',
        version: 2,
        external_id: 'job-1',
        title: 'Senior Data Engineer',
        url: 'https://fixture.test/jobs/1',
        location: 'Remote',
        salary: '$160,000 - $190,000',
        description: '<p>Build pipelines</p>',
        date_posted: '2026-10-15',
        company: 'Acme Corp',
        employer_logo_url: 'https://fixture.test/logo.png'
    }));
    console.log(JSON.stringify({
        type: 'candidate',
        version: 2,
        external_id: 'job-2',
        title: 'Staff Rust Engineer',
        url: 'https://fixture.test/jobs/2',
        location: 'San Francisco, CA',
        salary: '$190,000 - $220,000',
        description: '<p>High throughput systems</p>',
        date_posted: '2026-10-16',
        company: 'Acme Corp',
        employer_logo_url: null
    }));
    console.log(JSON.stringify({
        type: 'continuation',
        version: 2,
        cursor: 'page=2'
    }));
    console.log(JSON.stringify({
        type: 'complete',
        version: 2,
        exhausted: false
    }));
} else if (input.continuation === 'page=2') {
    // Turn 2
    console.log(JSON.stringify({
        type: 'candidate',
        version: 2,
        external_id: 'job-3',
        title: 'Principal Distributed Systems Engineer',
        url: 'https://fixture.test/jobs/3',
        location: 'Remote',
        salary: '$230,000 - $270,000',
        description: '<p>Architect distributed services</p>',
        date_posted: '2026-10-17',
        company: 'Acme Corp',
        employer_logo_url: null
    }));
    console.log(JSON.stringify({
        type: 'complete',
        version: 2,
        exhausted: true
    }));
}
"#;

    let config = json!({
        "version": scrape_core::PROTOCOL_VERSION,
        "sources": [{
            "id": "script-multi-turn",
            "name": "Acme Careers",
            "kind": "selector",
            "identifier": "https://fixture.test/jobs",
            "config": {
                "mode": "script",
                "script": {
                    "runtime": "js",
                    "code": script_code
                }
            }
        }],
        "filters": {
            "include_keywords": ["Engineer"]
        }
    });

    let (output, events) = run_worker_with_config(config);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let titles: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            WorkerEvent::Job { title, .. } => Some(title.clone()),
            _ => None,
        })
        .collect();

    assert_eq!(
        titles,
        vec![
            "Senior Data Engineer",
            "Staff Rust Engineer",
            "Principal Distributed Systems Engineer"
        ]
    );

    // Verify diagnostic was surfaced as warning or source info
    let diagnostics: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            WorkerEvent::Warning { message, .. } => Some(message.clone()),
            _ => None,
        })
        .collect();
    assert!(diagnostics
        .iter()
        .any(|d| d.contains("Started turn 1 crawl")));

    // Verify completion
    let has_done = events.iter().any(|e| matches!(e, WorkerEvent::Done { .. }));
    assert!(has_done);
}

#[test]
fn script_protocol_violations_fail_explicitly() {
    // 1. Malformed JSON on stdout
    let malformed_code = r#"
console.log("Not valid json frame");
"#;
    let config = json!({
        "version": scrape_core::PROTOCOL_VERSION,
        "sources": [{
            "id": "script-malformed",
            "name": "Acme",
            "kind": "selector",
            "identifier": "https://fixture.test/jobs",
            "config": {
                "mode": "script",
                "script": {
                    "runtime": "js",
                    "code": malformed_code
                }
            }
        }]
    });
    let (_, events) = run_worker_with_config(config);
    let error = events.iter().find_map(|e| match e {
        WorkerEvent::Error { message, .. } => Some(message.clone()),
        _ => None,
    });
    assert!(
        error
            .as_deref()
            .is_some_and(|msg| msg.contains("invalid NDJSON")),
        "Expected invalid NDJSON error, got {error:?}"
    );

    // 2. Unsupported protocol version
    let bad_version_code = r#"
console.log(JSON.stringify({ type: 'complete', version: 99, exhausted: true }));
"#;
    let config = json!({
        "version": scrape_core::PROTOCOL_VERSION,
        "sources": [{
            "id": "script-bad-version",
            "name": "Acme",
            "kind": "selector",
            "identifier": "https://fixture.test/jobs",
            "config": {
                "mode": "script",
                "script": {
                    "runtime": "js",
                    "code": bad_version_code
                }
            }
        }]
    });
    let (_, events) = run_worker_with_config(config);
    let error = events.iter().find_map(|e| match e {
        WorkerEvent::Error { message, .. } => Some(message.clone()),
        _ => None,
    });
    assert!(
        error
            .as_deref()
            .is_some_and(|msg| msg.contains("unsupported protocol version 99; expected 2")),
        "Expected protocol version mismatch error, got {error:?}"
    );

    // 3. Script exits 0 without emitting a complete frame
    let incomplete_code = r#"
console.log(JSON.stringify({
    type: 'candidate',
    version: 2,
    external_id: 'job-1',
    title: 'Staff Engineer',
    url: 'https://fixture.test/jobs/1'
}));
"#;
    let config = json!({
        "version": scrape_core::PROTOCOL_VERSION,
        "sources": [{
            "id": "script-incomplete",
            "name": "Acme",
            "kind": "selector",
            "identifier": "https://fixture.test/jobs",
            "config": {
                "mode": "script",
                "script": {
                    "runtime": "js",
                    "code": incomplete_code
                }
            }
        }]
    });
    let (_, events) = run_worker_with_config(config);
    let error = events.iter().find_map(|e| match e {
        WorkerEvent::Error { message, .. } => Some(message.clone()),
        _ => None,
    });
    assert!(
        error
            .as_deref()
            .is_some_and(|msg| msg.contains("without emitting a complete frame")),
        "Expected missing complete frame error, got {error:?}"
    );
}

#[test]
fn script_subprocess_failure_with_redacted_stderr_and_truncation() {
    // 1. Non-zero exit with sensitive tokens in stderr
    let sensitive_code = r#"
console.error("Error encountered: Cookie: session=super_secret_cookie_token_123");
console.error("Authorization: Bearer secret_api_key_456");
console.error("api_key: secret_client_token");
console.error("password: secret_password_789");
process.exit(1);
"#;
    let config = json!({
        "version": scrape_core::PROTOCOL_VERSION,
        "sources": [{
            "id": "script-sensitive",
            "name": "Acme",
            "kind": "selector",
            "identifier": "https://fixture.test/jobs",
            "config": {
                "mode": "script",
                "script": {
                    "runtime": "js",
                    "code": sensitive_code
                }
            }
        }]
    });
    let (_, events) = run_worker_with_config(config);
    let error = events.iter().find_map(|e| match e {
        WorkerEvent::Error { message, .. } => Some(message.clone()),
        _ => None,
    });
    assert!(error.is_some());
    let err_msg = error.unwrap();
    assert!(err_msg.contains("non-zero exit status"));
    assert!(err_msg.contains("Cookie=[REDACTED]"));
    assert!(err_msg.contains("Authorization=[REDACTED]"));
    assert!(err_msg.contains("api_key=[REDACTED]"));
    assert!(err_msg.contains("password=[REDACTED]"));
    // Ensure secrets are never leaked
    assert!(!err_msg.contains("super_secret_cookie_token_123"));
    assert!(!err_msg.contains("secret_api_key_456"));
    assert!(!err_msg.contains("secret_client_token"));
    assert!(!err_msg.contains("secret_password_789"));

    // 2. Stderr exceeding 16 KiB is truncated
    let big_stderr_code = r#"
console.error("A".repeat(25000));
process.exit(2);
"#;
    let config = json!({
        "version": scrape_core::PROTOCOL_VERSION,
        "sources": [{
            "id": "script-big-stderr",
            "name": "Acme",
            "kind": "selector",
            "identifier": "https://fixture.test/jobs",
            "config": {
                "mode": "script",
                "script": {
                    "runtime": "js",
                    "code": big_stderr_code
                }
            }
        }]
    });
    let (_, events) = run_worker_with_config(config);
    let error = events.iter().find_map(|e| match e {
        WorkerEvent::Error { message, .. } => Some(message.clone()),
        _ => None,
    });
    assert!(error.is_some());
    let err_msg = error.unwrap();
    assert!(err_msg.contains("[stderr truncated to 16 KiB]"));
}

#[test]
fn script_timeout_kills_child_process_tree() {
    let hanging_code = r#"
// Hang forever
setTimeout(() => {}, 300000);
"#;
    let config = json!({
        "version": scrape_core::PROTOCOL_VERSION,
        "sources": [{
            "id": "script-hanging",
            "name": "Acme",
            "kind": "selector",
            "identifier": "https://fixture.test/jobs",
            "config": {
                "mode": "script",
                "script": {
                    "runtime": "js",
                    "code": hanging_code,
                    "timeout_s": 1
                }
            }
        }]
    });

    let start = std::time::Instant::now();
    let (_, events) = run_worker_with_config(config);
    let elapsed = start.elapsed();

    // Verify it terminated within bounded time (< 4s), proving the timeout worked and process was killed
    assert!(
        elapsed < Duration::from_secs(5),
        "Took too long to kill hanging script: {elapsed:?}"
    );

    let has_timeout_error = events.iter().any(|e| match e {
        WorkerEvent::Error { message, .. } => message.contains("time limit"),
        _ => false,
    });
    assert!(
        has_timeout_error,
        "Expected timeout error event in {events:?}"
    );
}

#[test]
fn script_v2_uses_typed_evidence_and_rejects_invalid_locations_without_acceptance_authority() {
    let code = r#"
const fs=require('fs'); const input=JSON.parse(fs.readFileSync(0,'utf8'));
if(input.version!==2) throw Error('expected version 2');
const emit=(v)=>console.log(JSON.stringify({version:2,...v}));
emit({type:'candidate', external_id:'valid',title:'Engineer',url:'https://fixture.test/valid',employment_type:'FullTime',workplace_type:'remote',locations:[{raw:'Westfield, IN (provider country US)',city:'Westfield',region_code:'US-IN',country_code:'US',is_remote:true}]});
emit({type:'candidate', external_id:'bad',title:'Engineer',url:'https://fixture.test/bad',workplace_type:'remote',locations:[{raw:'Authored conflicting tuple',city:'Cupertino',region_code:'US-CA',country_code:'CA',is_remote:false}]});
emit({type:'candidate', external_id:'remote-only',title:'Engineer',url:'https://fixture.test/remote-only',workplace_type:'remote'});
emit({type:'candidate', external_id:'unknown',title:'Engineer',url:'https://fixture.test/unknown',location:'Europe'});
emit({type:'candidate', external_id:'split',title:'Engineer',url:'https://fixture.test/split',locations:[{raw:'Toronto',city:'Toronto',region_code:'CA-ON',country_code:'CA',is_remote:false},{raw:'Cupertino',city:'Cupertino',region_code:'US-CA',country_code:'US',is_remote:false}]});
emit({type:'complete',exhausted:true});
"#;
    for (rules, expected) in [
        (
            json!({"workplaces":["remote"]}),
            vec!["valid", "remote-only"],
        ),
        (
            json!({"role_types":["full_time"],"workplaces":["remote"],"countries":["US"],"regions":["US-IN"],"cities":["Westfield"]}),
            vec!["valid"],
        ),
        (json!({"countries":["CA"],"cities":["Cupertino"]}), vec![]),
    ] {
        let (_, events) = run_worker_with_config(
            json!({"version":scrape_core::PROTOCOL_VERSION,"sources":[{"id":"script-evidence","kind":"selector","name":"Authored evidence fixture","identifier":"https://fixture.test/jobs","config":{"mode":"script","script":{"runtime":"js","code":code}}}],"filters":{"matching":rules}}),
        );
        let jobs: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let WorkerEvent::Job { external_id, .. } = e {
                    Some(external_id.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(jobs, expected, "{events:?}");
        assert!(events.iter().any(|e|matches!(e,WorkerEvent::Error{invalid_posting:Some(true),message,..} if message.contains("invalid geographic evidence"))));
        let WorkerEvent::Done { stats } = events.last().unwrap() else {
            panic!()
        };
        assert_eq!(stats.sources[0].invalid, 1);
    }
    for frame in [
        json!({"type":"candidate","version":1,"title":"Engineer","url":"https://fixture.test/old"}),
        json!({"type":"candidate","version":2,"title":"Engineer","url":"https://fixture.test/bad","workplace_type":"invented"}),
        json!({"type":"candidate","version":2,"title":"Engineer","url":"https://fixture.test/bad","accepted":true}),
    ] {
        let code = format!("console.log({:?});", frame.to_string());
        let (_, events) = run_worker_with_config(
            json!({"version":scrape_core::PROTOCOL_VERSION,"sources":[{"id":"invalid-script","kind":"selector","name":"Invalid fixture","identifier":"https://fixture.test/jobs","config":{"mode":"script","script":{"runtime":"js","code":code}}}]}),
        );
        assert!(!events.iter().any(|e| matches!(e, WorkerEvent::Job { .. })));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, WorkerEvent::Error { .. })),
            "{events:?}"
        );
    }
}
