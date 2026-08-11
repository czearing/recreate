//! The single owner of "what filename extension does this asset's bytes need?".
//!
//! Every asset is stored under its content hash, so the extension carries no identity —
//! it exists only because the recreation is served by a static server that derives the
//! response's `Content-Type` from it. An asset written with the wrong extension is served
//! with the wrong type, and a consumer that parses by type rather than by sniffing — a
//! `<track>`'s WebVTT parser, an `@font-face` source — rejects it. So the file is present
//! in the artifact and still unusable, which a file-count check reports as success.
//!
//! The rule had been written twice, once for `data:` URLs and once for network responses,
//! and the two tables had drifted: the data table knew JPEG and OpenType, the network
//! table knew GIF, AVIF and MP4, and neither knew WebVTT. Naming the pairs once and
//! reading them in both directions is what keeps a type added for one route available to
//! the other.

/// Media type to extension. Read forwards to answer a response's `Content-Type`, and
/// backwards to answer a URL whose server declared nothing usable. A type may appear
/// twice: the first row wins the forward lookup, and the later row exists so the
/// alternate spelling of the same extension is recognised in a path.
const EXTENSIONS: [(&str, &str); 13] = [
    ("image/png", "png"),
    ("image/jpeg", "jpg"),
    ("image/webp", "webp"),
    ("image/gif", "gif"),
    ("image/avif", "avif"),
    ("image/svg+xml", "svg"),
    ("video/mp4", "mp4"),
    ("text/vtt", "vtt"),
    ("font/woff2", "woff2"),
    ("font/woff", "woff"),
    ("font/ttf", "ttf"),
    ("font/otf", "otf"),
    ("image/jpeg", "jpeg"),
];

/// Alternate media types for a type already in the table. These are the pre-registration
/// spellings still served by older origins; they name the same bytes.
const ALIASES: [(&str, &str); 6] = [
    ("application/font-woff2", "font/woff2"),
    ("application/font-woff", "font/woff"),
    ("application/x-font-ttf", "font/ttf"),
    ("application/x-font-opentype", "font/otf"),
    ("font/truetype", "font/ttf"),
    ("font/opentype", "font/otf"),
];

/// The extension for bytes described by `media_type`, falling back to what `path` claims
/// when the type is absent or generic. A query string is not part of the path and is
/// dropped before it is read, so a versioned URL still names its own format. `bin` is the
/// honest answer when neither source says anything: a wrong extension would assert a type
/// the bytes may not have, which a static server would then declare.
pub fn resolve(media_type: &str, path: &str) -> &'static str {
    let media_type = media_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let media_type = ALIASES
        .iter()
        .find(|(alias, _)| *alias == media_type)
        .map(|(_, canonical)| *canonical)
        .unwrap_or(&media_type);
    if let Some((_, extension)) = EXTENSIONS.iter().find(|(name, _)| *name == media_type) {
        return extension;
    }
    let path = path
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    EXTENSIONS
        .iter()
        .find(|(_, extension)| path.ends_with(&format!(".{extension}")))
        .map(|(_, extension)| *extension)
        .unwrap_or("bin")
}

#[cfg(test)]
#[path = "asset_extension_tests.rs"]
mod tests;
