//! Employer branding evidence and bounded, portable local assets.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use ts_rs::TS;

pub const MAX_LOGO_BYTES: usize = 512 * 1024;
pub const MAX_LOGO_DIMENSION: u32 = 2048;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LogoMethod {
    Provider,
    JobPosting,
    EmployerLabel,
    ConfiguredSelector,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct LogoEvidence {
    pub url: String,
    pub page_url: String,
    pub method: LogoMethod,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LogoStatus {
    Verified,
    MissingMetadata,
    DownloadFailed,
    DecodeFailed,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct CompanyLogo {
    pub evidence: Option<LogoEvidence>,
    pub checked_at: String,
    pub status: LogoStatus,
    pub asset: Option<String>,
    pub diagnostic: Option<String>,
}
/// No domain guesses or platform identities. Relationship keys survive source deletion.
pub fn relationship_key(source_id: &str, company: &str) -> String {
    serde_json::to_string(&(source_id, company.trim().to_lowercase())).expect("String tuple")
}
pub fn http_url(raw: &str) -> bool {
    url::Url::parse(raw).is_ok_and(|u| {
        matches!(u.scheme(), "http" | "https")
            && u.host_str().is_some()
            && u.username().is_empty()
            && u.password().is_none()
    })
}
impl LogoEvidence {
    pub fn validate(&self) -> Result<(), String> {
        if !http_url(&self.url) || !http_url(&self.page_url) {
            return Err(
                "Logo evidence requires original HTTP(S) asset and page URLs without credentials"
                    .into(),
            );
        }
        Ok(())
    }
}
impl CompanyLogo {
    pub fn validate(&self) -> Result<(), String> {
        chrono::DateTime::parse_from_rfc3339(&self.checked_at)
            .map_err(|_| "Logo check timestamp is invalid")?;
        if let Some(evidence) = &self.evidence {
            evidence.validate()?;
        }
        if self.status == LogoStatus::Verified {
            if self.evidence.is_none() {
                return Err("Verified logo lacks source evidence".into());
            }
            let asset = self
                .asset
                .as_deref()
                .ok_or("Verified logo lacks a cached asset")?;
            let raw = asset
                .strip_prefix("data:image/png;base64,")
                .ok_or("Logo cache must contain a PNG")?;
            if raw.len() > MAX_LOGO_BYTES * 2 {
                return Err("Logo cache exceeds byte limit".into());
            }
            let bytes = STANDARD.decode(raw).map_err(|_| "Malformed logo cache")?;
            if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                return Err("Logo cache is not PNG content".into());
            }
            decode_raster(&bytes)?;
        } else if self.asset.is_some() {
            return Err("Unverified logo cannot supply a cached asset".into());
        }
        Ok(())
    }
    pub fn asset_reference(&self) -> Option<String> {
        use sha2::{Digest, Sha256};
        self.asset
            .as_ref()
            .map(|asset| format!("sha256:{:x}", Sha256::digest(asset.as_bytes())))
    }
}
fn decode_raster(bytes: &[u8]) -> Result<image::DynamicImage, String> {
    if bytes.len() > MAX_LOGO_BYTES {
        return Err("Logo exceeds 512 KiB".into());
    }
    let mut reader = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(MAX_LOGO_DIMENSION);
    limits.max_image_height = Some(MAX_LOGO_DIMENSION);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|e| format!("Logo decoding failed: {e}"))
}
/// Canonical PNGs need no network, scripts, fonts or external SVG resources to display.
pub fn cache_asset(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > MAX_LOGO_BYTES {
        return Err("Logo exceeds 512 KiB".into());
    }
    let png = if let Ok(image) = decode_raster(bytes) {
        if image.to_rgba8().pixels().all(|pixel| pixel.0[3] == 0) {
            return Err("Logo contains no visible artwork".into());
        }
        let mut output = Cursor::new(Vec::new());
        image
            .write_to(&mut output, image::ImageOutputFormat::Png)
            .map_err(|e| e.to_string())?;
        output.into_inner()
    } else {
        let mut options = resvg::usvg::Options::default();
        options.image_href_resolver = resvg::usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        };
        let tree = resvg::usvg::Tree::from_data(bytes, &options)
            .map_err(|e| format!("Logo is not a decodable PNG, JPEG, WebP or SVG: {e}"))?;
        let size = tree.size().to_int_size();
        if size.width() > MAX_LOGO_DIMENSION || size.height() > MAX_LOGO_DIMENSION {
            return Err("SVG logo exceeds dimension limit".into());
        }
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
            .ok_or("Invalid SVG logo dimensions")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
        if pixmap.data().chunks_exact(4).all(|p| p[3] == 0) {
            return Err("SVG logo contains no visible self-contained artwork".into());
        }
        pixmap.encode_png().map_err(|e| e.to_string())?
    };
    if png.len() > MAX_LOGO_BYTES {
        return Err("Decoded logo exceeds cache byte limit".into());
    }
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_pixels_and_blocks_missing_external_svg_artwork() {
        let svg=br##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="20"><path fill="#f80" d="M0 0h80v20H0z"/></svg>"##;
        let asset = cache_asset(svg).unwrap();
        let report = CompanyLogo {
            evidence: Some(LogoEvidence {
                url: "https://fixture.test/logo.svg".into(),
                page_url: "https://fixture.test/jobs".into(),
                method: LogoMethod::EmployerLabel,
            }),
            checked_at: "2026-10-16T00:00:00Z".into(),
            status: LogoStatus::Verified,
            asset: Some(asset),
            diagnostic: None,
        };
        report.validate().unwrap();
        assert!(report.asset_reference().unwrap().starts_with("sha256:"));
        assert!(cache_asset(b"not an image").is_err());
        assert!(cache_asset(&vec![0; MAX_LOGO_BYTES + 1]).is_err());
        assert!(cache_asset(
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="9000" height="9000"/>"#
        )
        .is_err());
        assert!(cache_asset(br#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><image href="file:///secret" width="20" height="20"/></svg>"#).is_err());
    }
}
