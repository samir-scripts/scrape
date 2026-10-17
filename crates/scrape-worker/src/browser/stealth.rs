//! Pre-document stealth injection and heavy resource blocking patterns for CDP.
use serde_json::{json, Value};

pub const STEALTH_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

pub const STEALTH_INIT_SCRIPT: &str = r#"(() => {
  Object.defineProperty(navigator, 'webdriver', { get: () => false });
  if (!window.chrome) { window.chrome = { runtime: {} }; }
  Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });
  Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3] });
})();"#;

pub const BLOCKED_RESOURCE_URL_PATTERNS: &[&str] = &[
    "*.png",
    "*.jpg",
    "*.jpeg",
    "*.gif",
    "*.webp",
    "*.avif",
    "*.ico",
    "*.woff",
    "*.woff2",
    "*.ttf",
    "*.otf",
    "*.css",
    "*.mp4",
    "*.webm",
    "*.mp3",
    "*.wav",
    "*google-analytics.com*",
    "*googletagmanager.com*",
    "*doubleclick.net*",
    "*hotjar.com*",
    "*segment.io*",
    "*sentry.io*",
];

pub fn user_agent_override_params() -> Value {
    json!({
        "userAgent": STEALTH_USER_AGENT,
        "acceptLanguage": "en-US,en;q=0.9",
        "platform": "MacIntel",
        "userAgentMetadata": {
            "brands": [
                { "brand": "Not?A_Brand", "version": "99" },
                { "brand": "Chromium", "version": "130" },
                { "brand": "Google Chrome", "version": "130" }
            ],
            "fullVersion": "130.0.6723.58",
            "platform": "macOS",
            "platformVersion": "14.5.0",
            "architecture": "arm",
            "model": "",
            "mobile": false
        }
    })
}
