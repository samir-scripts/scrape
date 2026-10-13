//! Expected initialization failures must never unwind through the macOS launch callback.
use tauri::Manager;

pub fn failure(app: &mut tauri::App, message: &str) {
    eprintln!("STARTUP_FAILED: {message}");
    tracing::error!("Startup failed: {message}");
    // Process regressions require an observable controlled nonzero exit.
    if super::native_smoke::directory().is_some() {
        app.handle().exit(1);
        return;
    }
    let escaped = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let html = format!(
        r#"<!doctype html><html><meta charset="utf-8"><title>Scrape startup recovery</title>
<body style="background:#F5F5DC;color:#1A1A1A;font:16px system-ui;padding:32px;line-height:1.5">
<h1>Scrape could not start</h1><p>{escaped}</p>
<p>Your database was not reset. Preserve it and its WAL files before attempting recovery.</p>
<p>See <code>plans/database-recovery.md</code>. Close this window to quit.</p></body></html>"#
    );
    if let Some(window) = app.get_webview_window("main") {
        let url = format!(
            "data:text/html,{}",
            url::form_urlencoded::byte_serialize(html.as_bytes()).collect::<String>()
        );
        let displayed = url::Url::parse(&url)
            .ok()
            .is_some_and(|url| window.navigate(url).is_ok());
        if displayed {
            let _ = window.set_title("Scrape — startup recovery");
            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                    handle.exit(1);
                }
            });
            let _ = window.show();
            let _ = window.set_focus();
            return;
        }
    }
    app.handle().exit(1);
}
