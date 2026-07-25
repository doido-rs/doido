//! Asset tag helpers (Rails `image_tag`/`stylesheet_link_tag`/`javascript_include_tag`).

use super::escape;

/// Resolve an asset reference to a URL: absolute URLs / rooted paths pass
/// through; a bare name is served from `/assets/`.
fn asset_path(name: &str) -> String {
    if name.starts_with('/') || name.starts_with("http://") || name.starts_with("https://") {
        name.to_string()
    } else {
        format!("/assets/{name}")
    }
}

/// `<img src="/assets/logo.png">`.
pub fn image_tag(src: &str) -> String {
    format!("<img src=\"{}\">", escape(&asset_path(src)))
}

/// An image with `alt` text.
pub fn image_tag_alt(src: &str, alt: &str) -> String {
    format!(
        "<img src=\"{}\" alt=\"{}\">",
        escape(&asset_path(src)),
        escape(alt)
    )
}

/// `<link rel="stylesheet" href="/assets/application.css">`.
pub fn stylesheet_link_tag(name: &str) -> String {
    let href = if name.ends_with(".css") {
        asset_path(name)
    } else {
        asset_path(&format!("{name}.css"))
    };
    format!("<link rel=\"stylesheet\" href=\"{}\">", escape(&href))
}

/// `<script src="/assets/application.js"></script>`.
pub fn javascript_include_tag(name: &str) -> String {
    let src = if name.ends_with(".js") {
        asset_path(name)
    } else {
        asset_path(&format!("{name}.js"))
    };
    format!("<script src=\"{}\"></script>", escape(&src))
}

/// A content digest for cache-busting (Propshaft-style fingerprint).
pub fn digest(content: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// A digested asset path: `digested_path("app.css", b"...")` →
/// `/assets/app-<digest>.css`, so changed assets get a fresh URL.
pub fn digested_path(name: &str, content: &[u8]) -> String {
    let d = digest(content);
    match name.rsplit_once('.') {
        Some((stem, ext)) => format!("/assets/{stem}-{d}.{ext}"),
        None => format!("/assets/{name}-{d}"),
    }
}

/// An `<script type="importmap">` tag pinning module names to URLs
/// (importmap-rails). `importmap(&[("app", "/assets/app.js")])`.
pub fn importmap(pins: &[(&str, &str)]) -> String {
    let imports: Vec<String> = pins
        .iter()
        .map(|(name, url)| format!("\"{}\":\"{}\"", escape(name), escape(url)))
        .collect();
    format!(
        "<script type=\"importmap\">{{\"imports\":{{{}}}}}</script>",
        imports.join(",")
    )
}
