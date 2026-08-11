//! The single owner of "what does this element paint that nothing in the tree describes?".
//!
//! A drawing surface's bitmap lives in a buffer the document never mentions: serialising
//! the element yields its tag and its dimensions, and its content is reachable only by
//! reading the live element. Every other stage of this crate already knows such an element
//! — it is an atomic flex item, it is measured intrinsically, it is a replaced box — and
//! all of that decides how much room it occupies. None of it turns the content into bytes,
//! so without this the artifact emits a correctly sized, correctly bordered empty
//! rectangle and asserts that is what stood there.
//!
//! The family is named by capability rather than by tag. An element that can export what
//! it painted is exactly an element whose content is not in the tree; a tag list would
//! answer the same question for one spelling and be wrong for the next.
//!
//! Two reads return nothing while appearing to succeed, and both must be treated as
//! absence rather than as content: a surface that was never drawn on, and one whose
//! drawing buffer was discarded after the frame was presented, which is the default for
//! WebGL. Both export the bytes an unused surface of the same size exports, so that is
//! what they are compared against — no threshold, no sampling. A third read fails loudly:
//! a surface holding cross-origin pixels refuses to export at all. That is ordinary on
//! real pages, so it is recorded as a capture blocker and the capture continues; the
//! element is still emitted with its box, exactly as it is today.
//!
//! The recorded value is a key, not the bytes. Written into the asset table it travels the
//! path an inlined subresource already travels — decoded, hashed, written once, and the
//! reference repointed at the file — so a surface needs no asset concept of its own and
//! two elements that painted the same thing share one file.

/// The attribute carrying the key of an element's painted content. Read here and at the
/// emitter, so the two cannot disagree about what marks a surface. It is never emitted:
/// generation either resolves it into the source of the element that paints it, or drops
/// it because the bytes did not reach the project. The name is not `data-recreate-surface`,
/// which the generated runtime already uses for an open interaction surface.
pub const ATTRIBUTE: &str = "data-recreate-painted";

/// The key's scheme. Nothing fetches it — it names bytes the capture already holds — so it
/// only has to be a token no URL can equal and no whitespace can split.
const SCHEME: &str = "recreate-surface:";

/// Defines `recreateSurfaceAttributes(element, path)`, `recreateSurfaceAssets()` and
/// `recreateSurfaceBlockers()` for the injected capture scripts.
pub fn js_source() -> String {
    JS_SOURCE
        .replace("__SURFACE_ATTRIBUTE__", ATTRIBUTE)
        .replace("__SURFACE_SCHEME__", SCHEME)
}

const JS_SOURCE: &str = include_str!("surface_content.js");

#[cfg(test)]
#[path = "surface_content_tests.rs"]
mod tests;
