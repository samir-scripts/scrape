# Provider fixtures

`manifest.json` identifies raw, timestamped public provider captures by their capture URL and SHA-256. Files under `regressions/` may instead be authored deterministic edge cases. Each authored case must declare `fixture_kind`, keep `source_capture` null, and must never be described or tested as a live provider response.

`regressions/remote-us-availability-authored.json` is an authored Greenhouse-shaped case for a Remote posting whose eligible geography appears only in “Available Locations”, paired with a negative company-prose case. It does not claim to reproduce Cloudflare's raw API payload. `regressions/cloudflare-8024849.json` remains an evidence envelope for the user's reported posting; its current public API bytes could not be captured from the implementation environment.
