//! Takes one picture of the primary screen, shrunk and compressed so it can travel to a vision model.
//! Only reached after the `screen` power has been checked; the picture is returned, never stored.

use base64::Engine;
use xcap::image::{
    codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, ExtendedColorType,
};
use xcap::Monitor;

/// Widest picture sent, in pixels: enough to read a window, small enough to be cheap.
const MAX_WIDTH: u32 = 1280;
const QUALITY: u8 = 70;

/// The widest a picture may get before its base64 text would exceed what the brain accepts.
fn fits(jpeg_len: usize) -> bool {
    jpeg_len.div_ceil(3) * 4 <= slimy_brain::MAX_IMAGE
}

/// A JPEG of the image no wider than `width`.
fn encode(image: &DynamicImage, width: u32) -> Result<Vec<u8>, String> {
    let small = if image.width() > width {
        image.resize(width, u32::MAX, FilterType::Triangle)
    } else {
        image.clone()
    };
    let rgb = small.to_rgb8();
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, QUALITY)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|_| "screen_failed".to_string())?;
    Ok(bytes)
}

/// The picture as base64 JPEG text, narrowed step by step until it fits.
pub fn prepare(image: &DynamicImage) -> Result<String, String> {
    let mut width = MAX_WIDTH;
    loop {
        let bytes = encode(image, width)?;
        if fits(bytes.len()) {
            return Ok(base64::engine::general_purpose::STANDARD.encode(bytes));
        }
        width = width * 3 / 4;
        if width < 320 {
            return Err("screen_failed".into());
        }
    }
}

/// Captures the primary monitor with the system capture library (Windows, macOS, X11).
fn capture_monitor() -> Result<String, String> {
    let monitors = Monitor::all().map_err(|_| "screen_failed".to_string())?;
    let monitor = monitors
        .iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| monitors.first())
        .ok_or_else(|| "screen_failed".to_string())?;
    let shot = monitor
        .capture_image()
        .map_err(|_| "screen_failed".to_string())?;
    prepare(&DynamicImage::ImageRgba8(shot))
}

/// A `file://` address as a path (spaces and other characters are percent-encoded in it).
#[cfg(target_os = "linux")]
fn path_of(uri: &str) -> Option<std::path::PathBuf> {
    let raw = uri.strip_prefix("file://")?;
    let mut out = Vec::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            out.push(u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(std::path::PathBuf::from(String::from_utf8(out).ok()?))
}

/// On Wayland an application cannot read the screen: the desktop's Screenshot portal takes the picture, after its own
/// permission dialog the first time. The temporary file it writes is read once and deleted.
#[cfg(target_os = "linux")]
async fn capture_portal() -> Result<String, String> {
    use ashpd::desktop::screenshot::Screenshot;
    let asked = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        Screenshot::request()
            .interactive(false)
            .modal(true)
            .send()
            .await?
            .response()
    })
    .await;
    let shot = match asked {
        Ok(Ok(shot)) => shot,
        Ok(Err(_)) => return Err("screen_denied".into()),
        Err(_) => return Err("screen_timeout".into()),
    };
    let path = path_of(&shot.uri().to_string()).ok_or_else(|| "screen_failed".to_string())?;
    let image = xcap::image::open(&path).map_err(|_| "screen_failed".to_string());
    let _ = std::fs::remove_file(&path);
    prepare(&image?)
}

/// One picture of the screen as base64 JPEG text.
pub async fn capture() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) {
        return capture_portal().await;
    }
    tauri::async_runtime::spawn_blocking(capture_monitor)
        .await
        .map_err(|_| "screen_failed".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_big_picture_is_shrunk_to_something_the_brain_accepts() {
        let mut noisy = xcap::image::RgbaImage::new(4000, 2000);
        for (i, p) in noisy.pixels_mut().enumerate() {
            let v = (i.wrapping_mul(2654435761) >> 7) as u8;
            *p = xcap::image::Rgba([v, v.wrapping_mul(3), v.wrapping_add(90), 255]);
        }
        let text = prepare(&DynamicImage::ImageRgba8(noisy)).unwrap();
        assert!(text.len() <= slimy_brain::MAX_IMAGE);
        assert!(
            text.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b)),
            "valid base64 for the brain"
        );
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&text)
            .unwrap();
        assert_eq!(&bytes[..2], &[0xFF, 0xD8], "a JPEG");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn portal_addresses_become_paths() {
        assert_eq!(
            path_of("file:///tmp/a%20b.png"),
            Some(std::path::PathBuf::from("/tmp/a b.png"))
        );
        assert_eq!(
            path_of("file:///tmp/plain.png"),
            Some(std::path::PathBuf::from("/tmp/plain.png"))
        );
        assert_eq!(path_of("https://example.com/x.png"), None);
        assert_eq!(
            path_of("file:///tmp/x%2"),
            Some(std::path::PathBuf::from("/tmp/x%2"))
        );
    }

    #[test]
    fn a_small_picture_keeps_its_size() {
        let small = DynamicImage::ImageRgba8(xcap::image::RgbaImage::new(300, 200));
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(prepare(&small).unwrap())
            .unwrap();
        let back = xcap::image::load_from_memory(&bytes).unwrap();
        assert_eq!((back.width(), back.height()), (300, 200));
    }
}
