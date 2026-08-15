//! The single owner of "which references does this fragment of CSS text name, and where".
//!
//! Reading a `url()` out of CSS text and writing one back are the same act performed in
//! two directions, and they had no common owner: the collector scanned for values while
//! the emitter matched spellings, so a reference either side could read was one the other
//! could miss. Both now go through one scanner that reports each value together with the
//! span it occupies, which is what lets a rewrite put a replacement back exactly where the
//! original stood without re-deciding where that was.
//!
//! Nothing here touches the DOM. It is text in and text out, which is why it can be
//! prepended to any injected script and exercised by a plain Node harness.

/// The JavaScript half, for the injected capture scripts. Defines `cssUrlTokens(text)`,
/// `cssUrls(text)` and `mapCssUrls(text, map)`.
pub fn js_source() -> &'static str {
    JS_SOURCE
}

const JS_SOURCE: &str = include_str!("css_url_text.js");
