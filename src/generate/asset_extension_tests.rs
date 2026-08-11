use super::resolve;

/// The extension exists so a static server declares the right `Content-Type`. A subresource
/// whose consumer parses by declared type rather than by sniffing — WebVTT is the strict
/// case — is silently rejected when the bytes land under `bin`, so every destination the
/// asset gate admits has to reach an extension the server can map back.
#[test]
fn names_every_destination_the_asset_gate_admits() {
    assert_eq!(resolve("text/vtt", "/captions.vtt"), "vtt");
    assert_eq!(resolve("image/png", "/raster.png"), "png");
    assert_eq!(resolve("video/mp4", "/clip.mp4"), "mp4");
    assert_eq!(resolve("image/svg+xml", "/icon.svg"), "svg");
}

/// The two routes into the asset store, `data:` URLs and network responses, had each
/// carried their own table and knew different types. One table read by both is the only
/// thing that stops a type added for one route from being missing on the other.
#[test]
fn answers_the_types_that_had_lived_in_only_one_of_the_two_tables() {
    for (media_type, extension) in [
        ("image/jpeg", "jpg"),
        ("font/otf", "otf"),
        ("image/gif", "gif"),
        ("image/avif", "avif"),
    ] {
        assert_eq!(resolve(media_type, ""), extension, "for {media_type}");
    }
}

/// A response carries parameters and arbitrary case, and older origins still ship the
/// pre-registration font types. All of them name the same bytes.
#[test]
fn reads_a_header_as_a_server_actually_writes_it() {
    assert_eq!(resolve("text/vtt; charset=utf-8", ""), "vtt");
    assert_eq!(resolve("IMAGE/PNG", ""), "png");
    assert_eq!(resolve("application/font-woff2", ""), "woff2");
    assert_eq!(resolve("font/opentype", ""), "otf");
}

/// A server that declares nothing usable leaves the path as the only evidence. Both
/// spellings of a JPEG are real in the wild and neither may fall through to `bin`.
#[test]
fn falls_back_to_the_path_only_when_the_type_says_nothing() {
    assert_eq!(resolve("application/octet-stream", "/a/b.woff2"), "woff2");
    assert_eq!(resolve("", "/a/b.JPEG"), "jpeg");
    assert_eq!(resolve("", "/a/b.jpg"), "jpg");
    assert_eq!(resolve("", "/a/b.woff2?v=3#frag"), "woff2");
}

/// Guessing an extension from nothing would assert a type the bytes may not have, which
/// is a worse failure than an undeclared one: the server would then serve them as that
/// type. A query string is not part of the path and must not be read as one.
#[test]
fn refuses_to_guess_when_neither_source_says_anything() {
    assert_eq!(resolve("application/octet-stream", "/download"), "bin");
    assert_eq!(resolve("", ""), "bin");
}
