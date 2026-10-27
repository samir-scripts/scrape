//! Debug-only native verification, explicitly opted into by scripts/native-smoke.mjs.
//! Production builds always use the normal application directory and startup.
use std::path::PathBuf;

pub fn directory() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os("SCRAPE_NATIVE_SMOKE_DIR") {
        let path = PathBuf::from(path)
            .canonicalize()
            .expect("Smoke directory must exist");
        let temporary = std::env::temp_dir()
            .canonicalize()
            .expect("Temporary directory must exist");
        assert!(
            path.starts_with(temporary)
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("scrape-native-smoke-")),
            "Native smoke must use its isolated temporary directory"
        );
        return Some(path);
    }
    None
}

pub fn install(app: &mut tauri::App) -> bool {
    let Some(directory) = directory() else {
        return false;
    };
    use tauri::{Listener, Manager};
    // A visible test window is not necessarily the active macOS application.
    // Exercise the same foreground lifecycle as opening Scrape from its tray.
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(window) = app.get_webview_window("main") {
        window.show().expect("Show isolated test window");
        window.set_focus().expect("Focus isolated test window");
        println!(
            "NATIVE_SMOKE_WINDOW visible={:?} focused={:?} minimized={:?}",
            window.is_visible(),
            window.is_focused(),
            window.is_minimized()
        );
    }
    let log_directory = directory.clone();
    app.listen("native-smoke-log", move |event| {
        use std::io::Write;
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_directory.join("workflow-progress.jsonl"))
            .expect("Open isolated native progress log");
        writeln!(log, "{}", event.payload()).expect("Persist isolated native progress");
        println!("NATIVE_SMOKE_LOG {}", event.payload());
    });
    let control_directory = directory.clone();
    let control_handle = app.handle().clone();
    app.listen("native-smoke-control", move |event| {
        let payload: serde_json::Value = serde_json::from_str(event.payload()).unwrap_or_default();
        let mode = payload["mode"].as_str().unwrap_or("");
        if mode == "resize-window" {
            let width = payload["width"].as_u64().unwrap_or(0);
            let height = payload["height"].as_u64().unwrap_or(0);
            if (400..=1600).contains(&width) && (480..=1000).contains(&height) {
                if let Some(window) = control_handle.get_webview_window("main") {
                    window
                        .set_size(tauri::LogicalSize::new(width as f64, height as f64))
                        .expect("Resize isolated test window");
                }
            }
        } else if mode == "restore-real-worker" {
            let _ = std::fs::remove_file(control_directory.join("worker-fixture"));
            let _ = std::fs::remove_file(control_directory.join("timeout-seconds"));
        } else if [
            "optional-warning",
            "required-evidence",
            "malformed",
            "wrong-source",
            "invalid-utf8",
            "invalid-branding",
            "nonzero",
            "missing-completion",
            "missing-summary",
            "duplicate-summary",
            "bad-sequence",
            "missing-evaluation",
            "wrong-evaluation-source",
            "wrong-job-correlation",
            "pending-eof",
            "timeout",
        ]
        .contains(&mode)
        {
            std::fs::write(control_directory.join("failure-mode"), mode).expect("Set fixture mode");
            std::fs::write(control_directory.join("timeout-seconds"), "2")
                .expect("Set bounded fixture timeout");
            std::fs::copy(
                control_directory.join("failure-worker.js"),
                control_directory.join("worker-fixture"),
            )
            .expect("Prepare scoped worker fixture");
        }
    });
    let result_directory = directory.clone();
    let handle = app.handle().clone();
    app.listen("native-smoke-result", move |event| {
        let result: serde_json::Value = serde_json::from_str(event.payload()).unwrap_or_default();
        std::fs::write(result_directory.join("result.json"), result.to_string())
            .expect("Persist native completion report");
        println!("NATIVE_SMOKE_RESULT {result}");
        handle.exit(if result["ok"] == true { 0 } else { 1 });
    });
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        // Native-clock probes distinguish a paused WebView timer from a blocked
        // JavaScript context without extending the existing overall deadline.
        for seconds in [15, 30] {
            tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
            if let Some(window) = handle.get_webview_window("main") {
                println!(
                    "NATIVE_SMOKE_WATCHDOG visible={:?} focused={:?} minimized={:?}",
                    window.is_visible(),
                    window.is_focused(),
                    window.is_minimized()
                );
                let _ = window.eval(
                    r#"window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {
                        event: "native-smoke-log",
                        payload: {
                            step: "native-clock probe",
                            workflow: window.__SCRAPE_NATIVE_WORKFLOW_STATE__ || null,
                            visibility: document.visibilityState,
                            focused: document.hasFocus(),
                            active: document.activeElement?.getAttribute("aria-label"),
                            country: document.querySelector('[aria-label="Country"]')?.value,
                            stateDisabled: document.querySelector('[aria-label="State/province"]')?.disabled
                        }
                    })"#,
                );
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        eprintln!("Native smoke deadline exceeded");
        handle.exit(1);
    });
    assert!(directory.join("smoke.js").is_file());
    true
}
