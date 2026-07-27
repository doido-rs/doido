//! Metadata analyzers (feature `storage-image`).
//!
//! The image analyzer extracts width/height without decoding the whole file
//! (via `imagesize`), the analogue of ActiveStorage's `ImageAnalyzer`. Video and
//! audio analyzers (which need ffmpeg) are deferred.

use serde_json::{json, Value};

/// Image pixel dimensions `(width, height)` if `data` is a recognizable image.
pub fn image_dimensions(data: &[u8]) -> Option<(usize, usize)> {
    imagesize::blob_size(data)
        .ok()
        .map(|size| (size.width, size.height))
}

/// Analyze `data` of type `content_type`, returning JSON metadata suitable for a
/// blob's `metadata` column. Currently fills `width`/`height` for images.
pub fn analyze(content_type: &str, data: &[u8]) -> Value {
    if crate::content_type::is_image(content_type) {
        if let Some((width, height)) = image_dimensions(data) {
            return json!({ "width": width, "height": height, "analyzed": true });
        }
    }
    json!({ "analyzed": true })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_png_dimensions() {
        // Minimal 1x1 PNG.
        let png: &[u8] = &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1f, 0x15, 0xc4, 0x89,
        ];
        assert_eq!(image_dimensions(png), Some((1, 1)));
        let meta = analyze("image/png", png);
        assert_eq!(meta["width"], 1);
        assert_eq!(meta["height"], 1);
    }
}
