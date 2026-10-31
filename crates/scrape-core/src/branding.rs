//! Employer branding evidence and bounded, portable local assets.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use ts_rs::TS;

pub const MAX_LOGO_BYTES: usize = 512 * 1024;
pub const MAX_LOGO_DIMENSION: u32 = 5120;
pub const MAX_CACHED_LOGO_DIMENSION: u32 = 128;
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
    // Read models use a content reference; worker reports carry validated bytes.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub cached_asset_reference: Option<String>,
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

/// Identifies the Ashby platform favicon previously misclassified as employer artwork.
pub fn is_legacy_ashby_platform_favicon(evidence: &LogoEvidence) -> bool {
    let Ok(page) = url::Url::parse(&evidence.page_url) else {
        return false;
    };
    let Ok(asset) = url::Url::parse(&evidence.url) else {
        return false;
    };
    if page.scheme() != "https"
        || page.host_str() != Some("jobs.ashbyhq.com")
        || asset.scheme() != "https"
        || asset.host_str() != Some("cdn.ashbyprd.com")
    {
        return false;
    }
    let Some(path) = asset.path().strip_prefix("/cdn_assets/") else {
        return false;
    };
    let Some((parent, filename)) = path.rsplit_once('/') else {
        return false;
    };
    !parent.is_empty()
        && filename
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("favicon."))
        && filename.len() > 8
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
    /// Validate worker report bytes before persistence. Read models expose a cache reference.
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
            let (raw, format) = if let Some(raw) = asset.strip_prefix("data:image/webp;base64,") {
                (raw, image::ImageFormat::WebP)
            } else if let Some(raw) = asset.strip_prefix("data:image/png;base64,") {
                // Previously persisted PNGs remain readable without changing their content hash.
                (raw, image::ImageFormat::Png)
            } else {
                return Err("Logo cache must contain a WebP or PNG".into());
            };
            if raw.len() > MAX_LOGO_BYTES * 2 {
                return Err("Logo cache exceeds byte limit".into());
            }
            let bytes = STANDARD.decode(raw).map_err(|_| "Malformed logo cache")?;
            if image::guess_format(&bytes).ok() != Some(format) {
                return Err("Logo cache MIME does not match its image content".into());
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
fn decode_raster(bytes: &[u8]) -> Result<image::RgbaImage, String> {
    if bytes.len() > MAX_LOGO_BYTES {
        return Err("Logo exceeds 512 KiB".into());
    }
    let mut reader = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(MAX_LOGO_DIMENSION);
    limits.max_image_height = Some(MAX_LOGO_DIMENSION);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|e| format!("Logo decoding failed: {e}"))?
        .into_rgba8();
    if image.pixels().all(|pixel| pixel.0[3] == 0) {
        return Err("Logo contains no visible artwork".into());
    }
    Ok(image)
}
fn cached_dimensions(width: u32, height: u32) -> (u32, u32) {
    let edge = width.max(height);
    if edge <= MAX_CACHED_LOGO_DIMENSION {
        return (width, height);
    }
    let scale = MAX_CACHED_LOGO_DIMENSION as f64 / edge as f64;
    (
        (width as f64 * scale).round().max(1.0) as u32,
        (height as f64 * scale).round().max(1.0) as u32,
    )
}
/// Small lossless WebP assets need no network, scripts, fonts or external SVG resources.
pub fn cache_asset(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > MAX_LOGO_BYTES {
        return Err("Logo exceeds 512 KiB".into());
    }
    let pixels = if image::guess_format(bytes).is_ok() {
        let image = decode_raster(bytes)?;
        let (width, height) = cached_dimensions(image.width(), image.height());
        if image.dimensions() == (width, height) {
            image
        } else {
            image::imageops::resize(&image, width, height, image::imageops::FilterType::Lanczos3)
        }
    } else {
        let options = resvg::usvg::Options {
            image_href_resolver: resvg::usvg::ImageHrefResolver {
                resolve_data: Box::new(|_, _, _| None),
                resolve_string: Box::new(|_, _| None),
            },
            ..Default::default()
        };
        let tree = resvg::usvg::Tree::from_data(bytes, &options)
            .map_err(|e| format!("Logo is not a decodable PNG, JPEG, WebP or SVG: {e}"))?;
        let size = tree.size();
        if size.width() > MAX_LOGO_DIMENSION as f32 || size.height() > MAX_LOGO_DIMENSION as f32 {
            return Err("SVG logo exceeds dimension limit".into());
        }
        let scale = (MAX_CACHED_LOGO_DIMENSION as f32 / size.width().max(size.height())).min(1.0);
        let width = (size.width() * scale).round().max(1.0) as u32;
        let height = (size.height() * scale).round().max(1.0) as u32;
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(width, height).ok_or("Invalid SVG logo dimensions")?;
        // Render directly at the cache size rather than allocating the full source bitmap.
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        if pixmap.data().chunks_exact(4).all(|p| p[3] == 0) {
            return Err("SVG logo contains no visible self-contained artwork".into());
        }
        // tiny-skia stores premultiplied RGBA; the image encoder expects straight alpha.
        for pixel in pixmap.data_mut().chunks_exact_mut(4) {
            if let Some(alpha) = std::num::NonZeroU32::new(pixel[3] as u32) {
                let a = alpha.get();
                for channel in &mut pixel[..3] {
                    *channel = ((*channel as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            } else {
                pixel[..3].fill(0);
            }
        }
        image::RgbaImage::from_raw(width, height, pixmap.take()).ok_or("Invalid SVG logo pixels")?
    };
    let mut webp = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut webp)
        .encode(
            pixels.as_raw(),
            pixels.width(),
            pixels.height(),
            image::ColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
    if webp.len() > MAX_LOGO_BYTES {
        return Err("Decoded logo exceeds cache byte limit".into());
    }
    Ok(format!("data:image/webp;base64,{}", STANDARD.encode(webp)))
}
#[cfg(test)]
mod tests {
    #[test]
    fn legacy_ashby_filter_matches_only_the_platform_favicon_relationship() {
        let evidence = |page: &str, asset: &str| LogoEvidence {
            url: asset.into(),
            page_url: page.into(),
            method: LogoMethod::EmployerLabel,
        };
        assert!(is_legacy_ashby_platform_favicon(&evidence(
            "https://jobs.ashbyhq.com/linear/123",
            "https://cdn.ashbyprd.com/cdn_assets/x/favicon.png"
        )));
        assert!(is_legacy_ashby_platform_favicon(&evidence(
            "https://jobs.ashbyhq.com/linear/123",
            "https://cdn.ashbyprd.com/cdn_assets/x/y/favicon.webp?v=2"
        )));
        for (page, asset) in [
            (
                "https://jobs.ashbyhq.com.evil.test/linear/123",
                "https://cdn.ashbyprd.com/cdn_assets/x/favicon.png",
            ),
            (
                "https://jobs.ashbyhq.com/linear/123",
                "https://cdn.ashbyprd.com.evil.test/cdn_assets/x/favicon.png",
            ),
            (
                "http://jobs.ashbyhq.com/linear/123",
                "https://cdn.ashbyprd.com/cdn_assets/x/favicon.png",
            ),
            (
                "https://jobs.ashbyhq.com/linear/123",
                "https://cdn.ashbyprd.com/cdn_assets/favicon.png",
            ),
            (
                "https://jobs.ashbyhq.com/linear/123",
                "https://cdn.ashbyprd.com/cdn_assets/x/logo.png",
            ),
            (
                "https://jobs.ashbyhq.com/linear/123",
                "https://cdn.ashbyprd.com/other/favicon.png",
            ),
            (
                "https://linear.app/careers",
                "https://cdn.ashbyprd.com/cdn_assets/x/favicon.png",
            ),
        ] {
            assert!(!is_legacy_ashby_platform_favicon(&evidence(page, asset)));
        }
    }
    use super::*;
    fn report(asset: String) -> CompanyLogo {
        CompanyLogo {
            evidence: Some(LogoEvidence {
                url: "https://fixture.test/logo.svg".into(),
                page_url: "https://fixture.test/jobs".into(),
                method: LogoMethod::EmployerLabel,
            }),
            checked_at: "2026-10-16T00:00:00Z".into(),
            status: LogoStatus::Verified,
            asset: Some(asset),
            cached_asset_reference: None,
            diagnostic: None,
        }
    }

    fn png(image: image::RgbaImage) -> Vec<u8> {
        let mut output = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut output, image::ImageOutputFormat::Png)
            .unwrap();
        output.into_inner()
    }

    fn cached_pixels(asset: &str) -> image::RgbaImage {
        let bytes = STANDARD
            .decode(asset.strip_prefix("data:image/webp;base64,").unwrap())
            .unwrap();
        assert_eq!(
            image::guess_format(&bytes).unwrap(),
            image::ImageFormat::WebP
        );
        report(asset.to_owned()).validate().unwrap();
        decode_raster(&bytes).unwrap()
    }

    #[test]
    fn small_raster_webp_is_lossless_with_alpha_and_is_not_upscaled() {
        let mut original = image::RgbaImage::new(40, 10);
        original.put_pixel(4, 4, image::Rgba([200, 80, 30, 128]));
        original.put_pixel(5, 4, image::Rgba([15, 90, 220, 255]));
        let pixels = cached_pixels(&cache_asset(&png(original.clone())).unwrap());
        assert_eq!(pixels.dimensions(), (40, 10));
        assert_eq!(pixels.get_pixel(0, 0).0[3], 0);
        assert_eq!(pixels.get_pixel(4, 4), original.get_pixel(4, 4));
        assert_eq!(pixels.get_pixel(5, 4), original.get_pixel(5, 4));
        // Already-encoded WebP input also follows the same bounded decode path.
        let asset = cache_asset(&png(original)).unwrap();
        let bytes = STANDARD.decode(asset.split_once(',').unwrap().1).unwrap();
        assert_eq!(cached_pixels(&cache_asset(&bytes).unwrap()), pixels);
    }

    #[test]
    fn raster_downscale_preserves_aspect_ratio_transparency_and_visible_artwork() {
        for (width, height, expected) in [(1024, 256, (128, 32)), (256, 1024, (32, 128))] {
            let original = image::RgbaImage::from_fn(width, height, |x, y| {
                if x < width / 4 || y < height / 4 {
                    image::Rgba([0, 0, 0, 0])
                } else {
                    image::Rgba([240, 120, 40, 192])
                }
            });
            let pixels = cached_pixels(&cache_asset(&png(original)).unwrap());
            assert_eq!(pixels.dimensions(), expected);
            assert_eq!(pixels.get_pixel(0, 0).0[3], 0);
            assert!(pixels.pixels().any(|pixel| pixel.0[3] > 0));
        }
    }

    #[test]
    fn svg_renders_at_target_size_and_unpremultiplies_alpha() {
        let small = br##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="20"><path fill="#c8501e" fill-opacity="0.5" d="M4 4h72v12H4z"/></svg>"##;
        let large = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="400" viewBox="0 0 80 20"><path fill="#c8501e" fill-opacity="0.5" d="M4 4h72v12H4z"/></svg>"##;
        for (svg, expected) in [(small.as_slice(), (80, 20)), (large.as_slice(), (128, 32))] {
            let pixels = cached_pixels(&cache_asset(svg).unwrap());
            assert_eq!(pixels.dimensions(), expected);
            assert_eq!(pixels.get_pixel(0, 0).0[3], 0);
            let color = pixels.get_pixel(expected.0 / 2, expected.1 / 2).0;
            for (actual, expected) in color.into_iter().zip([200_i16, 80, 30, 128]) {
                assert!((actual as i16 - expected).abs() <= 1, "{color:?}");
            }
        }
    }

    #[test]
    fn lightweight_high_resolution_logo_is_downscaled() {
        let bytes = png(image::RgbaImage::from_pixel(
            4167,
            1413,
            image::Rgba([255, 80, 0, 255]),
        ));
        assert!(bytes.len() < MAX_LOGO_BYTES);
        let pixels = cached_pixels(&cache_asset(&bytes).unwrap());
        assert_eq!(pixels.dimensions(), (128, 43));
        assert_eq!(pixels.get_pixel(64, 21).0[3], 255);
    }

    #[test]
    fn persisted_png_is_readable_without_resizing_or_changing_its_hash() {
        use sha2::{Digest, Sha256};
        let bytes = png(image::RgbaImage::from_pixel(
            256,
            64,
            image::Rgba([255, 80, 0, 255]),
        ));
        let asset = format!("data:image/png;base64,{}", STANDARD.encode(&bytes));
        let logo = report(asset.clone());
        let reference = format!("sha256:{:x}", Sha256::digest(asset.as_bytes()));
        logo.validate().unwrap();
        assert_eq!(logo.asset.as_deref(), Some(asset.as_str()));
        assert_eq!(logo.asset_reference().as_deref(), Some(reference.as_str()));
        assert_eq!(decode_raster(&bytes).unwrap().dimensions(), (256, 64));
        assert_eq!(
            cached_pixels(&cache_asset(&bytes).unwrap()).dimensions(),
            (128, 32)
        );
    }

    #[test]
    fn rejects_mime_mismatches_unsupported_cache_formats_and_invisible_artwork() {
        let bytes = png(image::RgbaImage::from_pixel(
            8,
            8,
            image::Rgba([0, 80, 255, 255]),
        ));
        assert!(report(format!(
            "data:image/webp;base64,{}",
            STANDARD.encode(&bytes)
        ))
        .validate()
        .is_err());
        let webp = cache_asset(&bytes).unwrap();
        assert!(report(webp.replacen("image/webp", "image/png", 1))
            .validate()
            .is_err());
        assert!(report(format!(
            "data:image/jpeg;base64,{}",
            STANDARD.encode(&bytes)
        ))
        .validate()
        .is_err());
        assert!(report("data:image/webp;base64,invalid".into())
            .validate()
            .is_err());
        assert!(cache_asset(&png(image::RgbaImage::new(8, 8))).is_err());
    }

    #[test]
    fn blocks_invalid_oversized_and_missing_external_svg_artwork() {
        assert!(cache_asset(b"not an image").is_err());
        assert!(cache_asset(&vec![0; MAX_LOGO_BYTES + 1]).is_err());
        let oversized = png(image::RgbaImage::from_pixel(
            MAX_LOGO_DIMENSION + 1,
            1,
            image::Rgba([255, 80, 0, 255]),
        ));
        assert!(cache_asset(&oversized).is_err());
        assert!(cache_asset(
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="9000" height="9000"/>"#
        )
        .is_err());
        for href in [
            "file:///secret",
            "https://fixture.test/external.png",
            "data:image/png;base64,AAAA",
        ] {
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><image href="{href}" width="20" height="20"/></svg>"#
            );
            assert!(cache_asset(svg.as_bytes()).is_err(), "{href}");
        }
    }
}
