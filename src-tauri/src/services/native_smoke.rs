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
    use tauri::Listener;
    let control_directory = directory.clone();
    app.listen("native-smoke-control", move |event| {
        let payload: serde_json::Value = serde_json::from_str(event.payload()).unwrap_or_default();
        let mode = payload["mode"].as_str().unwrap_or("");
        if mode == "restore-real-worker" {
            let _ = std::fs::remove_file(control_directory.join("worker-fixture"));
            let _ = std::fs::remove_file(control_directory.join("timeout-seconds"));
        } else if [
            "malformed",
            "wrong-source",
            "invalid-utf8",
            "nonzero",
            "missing-completion",
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
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        eprintln!("Native smoke deadline exceeded");
        handle.exit(1);
    });
    assert!(directory.join("smoke.js").is_file());
    true
}
