//! Content-type detection — a compact, dependency-free analogue of Rails' Marcel.
//!
//! [`detect`] sniffs a few well-known magic-byte signatures, then falls back to
//! the filename extension, then to `application/octet-stream`. The `is_*` helpers
//! mirror `blob.image?` / `video?` / `audio?` / `text?`.

/// The default type for unknown content.
pub const DEFAULT: &str = "application/octet-stream";

/// Detect the content type from the filename and the leading bytes.
pub fn detect(filename: &str, data: &[u8]) -> String {
    if let Some(t) = from_magic(data) {
        return t.to_string();
    }
    from_filename(filename).unwrap_or(DEFAULT).to_string()
}

/// Content type inferred from a filename extension, if recognized.
pub fn from_filename(filename: &str) -> Option<&'static str> {
    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let t = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "tif" | "tiff" => "image/tiff",
        "avif" => "image/avif",
        "heic" | "heif" => "image/heic",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "gz" | "gzip" => "application/gzip",
        "tar" => "application/x-tar",
        "json" => "application/json",
        "xml" => "application/xml",
        "csv" => "text/csv",
        "txt" | "text" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "text/javascript",
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => return None,
    };
    Some(t)
}

/// Content type inferred from magic bytes, for the common binary formats.
fn from_magic(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        return Some("image/png");
    }
    if data.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some("image/jpeg");
    }
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if data.starts_with(b"%PDF-") {
        return Some("application/pdf");
    }
    if data.starts_with(&[0x50, 0x4b, 0x03, 0x04]) {
        return Some("application/zip");
    }
    if data.starts_with(&[0x1f, 0x8b]) {
        return Some("application/gzip");
    }
    if data.starts_with(b"ID3") || data.starts_with(&[0xff, 0xfb]) {
        return Some("audio/mpeg");
    }
    None
}

/// Whether `content_type` is an image type.
pub fn is_image(content_type: &str) -> bool {
    content_type.starts_with("image/")
}

/// Whether `content_type` is a video type.
pub fn is_video(content_type: &str) -> bool {
    content_type.starts_with("video/")
}

/// Whether `content_type` is an audio type.
pub fn is_audio(content_type: &str) -> bool {
    content_type.starts_with("audio/")
}

/// Whether `content_type` is textual (covers `text/*` plus common structured
/// text types such as JSON and XML).
pub fn is_text(content_type: &str) -> bool {
    content_type.starts_with("text/")
        || matches!(
            content_type,
            "application/json" | "application/xml" | "image/svg+xml"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_wins_over_extension() {
        // PNG bytes but a .txt name → magic detection wins.
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        assert_eq!(detect("note.txt", &png), "image/png");
    }

    #[test]
    fn falls_back_to_extension_then_default() {
        assert_eq!(detect("report.pdf", b""), "application/pdf");
        assert_eq!(detect("mystery.unknownext", b""), DEFAULT);
    }

    #[test]
    fn type_predicates() {
        assert!(is_image("image/png"));
        assert!(is_video("video/mp4"));
        assert!(is_audio("audio/mpeg"));
        assert!(is_text("text/plain"));
        assert!(is_text("application/json"));
        assert!(!is_image("application/pdf"));
    }
}
