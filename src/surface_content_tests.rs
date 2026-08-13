//! What the capture reads off a drawing surface, run under Node against a scripted DOM.
//!
//! Three reads have to be told apart, and only one of them fails: content, a surface that
//! was never drawn on, and a surface holding cross-origin pixels. The first two are the
//! dangerous pair — both succeed, and one of them returns an image of nothing — so the
//! discriminator is exercised here rather than trusted.

use crate::node_eval;
use serde_json::{Value, json};

/// The reader under a DOM whose only behaviour is the platform's: a surface exports what
/// was drawn on it, and one made fresh at the same dimensions exports the blank bytes.
const HARNESS: &str = r#"
__SURFACE_CONTENT__
globalThis.document = {
  createElement: () => ({
    width: 0,
    height: 0,
    toDataURL() { return `blank(${this.width}x${this.height})`; }
  })
};
const surface = __SURFACE__;
const attributes = recreateSurfaceAttributes(surface, 'html>body>canvas:nth-of-type(1)');
console.log(JSON.stringify({
  attributes,
  assets: recreateSurfaceAssets(),
  blockers: recreateSurfaceBlockers(),
  probes: surface.probes || []
}));
"#;

fn read(surface: &str) -> Value {
    node_eval::json(
        &HARNESS
            .replace("__SURFACE_CONTENT__", &super::js_source())
            .replace("__SURFACE__", surface),
    )
}

fn painted(export: &str) -> String {
    format!("{{ width: 200, height: 120, toDataURL: () => '{export}' }}")
}

/// A surface that answers `context` to every context request and records what it was asked
/// for, so a test can pin both which context type decided the classification and whether
/// the surface was probed at all.
fn probed(export: &str, context: &str) -> String {
    format!(
        "{{ width: 200, height: 120, probes: [], toDataURL: () => '{export}', \
         getContext(type) {{ this.probes.push(type); return {context}; }} }}"
    )
}

/// The subject: content that exists nowhere in the tree is recorded, and the element is
/// left holding a key rather than the bytes, so two surfaces that painted the same thing
/// reach one file.
#[test]
fn records_what_was_painted_on_the_surface() {
    let result = read(&painted("data:image/png;base64,PAINTED"));
    let key = "recreate-surface:html>body>canvas:nth-of-type(1)";
    assert_eq!(result["attributes"], json!({ super::ATTRIBUTE: key }));
    assert_eq!(
        result["assets"],
        json!({ key: "data:image/png;base64,PAINTED" })
    );
    assert_eq!(result["blockers"], json!([]));
}

/// A surface that was never drawn on exports an image of nothing, at the right size, with
/// no error — the same bytes a discarded WebGL drawing buffer exports. Recording it would
/// publish a fabricated payload over content that never existed, so the two silent-blank
/// paths are closed by comparing against what a blank surface of these dimensions exports
/// rather than by any threshold.
#[test]
fn treats_an_unpainted_surface_as_no_content() {
    let result = read("{ width: 200, height: 120, toDataURL: () => 'blank(200x120)' }");
    assert_eq!(result["attributes"], json!({}));
    assert_eq!(result["assets"], json!({}));
    assert_eq!(result["blockers"], json!([]));
}

/// The blank comparison must be made at the surface's own dimensions. A surface painted
/// edge to edge in one colour can export exactly what a *differently* sized blank surface
/// exports, and dropping it would delete real content.
#[test]
fn compares_blankness_at_the_surface_dimensions() {
    let result = read("{ width: 200, height: 120, toDataURL: () => 'blank(64x64)' }");
    assert_eq!(
        result["attributes"],
        json!({ super::ATTRIBUTE: "recreate-surface:html>body>canvas:nth-of-type(1)" })
    );
}

/// A surface holding cross-origin pixels refuses to export. That is ordinary on real
/// pages, so the capture continues and the element keeps its box; what must not happen is
/// silence, because a recreation missing content nobody recorded is indistinguishable from
/// a page that had none.
#[test]
fn records_a_surface_it_is_not_allowed_to_read_as_a_blocker() {
    let result = read(
        "{ width: 200, height: 120, toDataURL: () => { \
         const error = new Error('tainted'); error.name = 'SecurityError'; throw error; } }",
    );
    assert_eq!(result["attributes"], json!({}));
    assert_eq!(result["assets"], json!({}));
    let blockers = result["blockers"].as_array().expect("blockers");
    assert_eq!(blockers.len(), 1);
    let blocker = blockers[0].as_str().unwrap_or_default();
    assert!(blocker.contains("SecurityError"), "{blocker}");
    assert!(
        blocker.contains("html>body>canvas:nth-of-type(1)"),
        "{blocker}"
    );
}

/// Every other element in the page passes through this call, so the family has to be
/// decided by the capability rather than by a tag list, and everything outside it has to
/// cost nothing.
#[test]
fn leaves_an_element_that_paints_no_surface_alone() {
    let result = read("{ tagName: 'DIV' }");
    assert_eq!(result["attributes"], json!({}));
    assert_eq!(result["assets"], json!({}));
    assert_eq!(result["blockers"], json!([]));
}

/// The subject. A surface whose drawing buffer was discarded exports the bytes of an unused
/// one, so the read succeeds and nothing throws — the failure never announces itself. It is
/// still a failed read: the user saw pixels the capture could not obtain, which is the same
/// premise the cross-origin blocker records. Contexts are exclusive, so a canvas already
/// bound to a non-2D context answers `null` here, and that is what separates it from a
/// surface nobody drew on.
#[test]
fn records_a_surface_whose_drawing_buffer_was_discarded_as_a_blocker() {
    let result = read(&probed("blank(200x120)", "null"));
    assert_eq!(result["attributes"], json!({}));
    assert_eq!(result["assets"], json!({}));
    assert_eq!(result["probes"], json!(["2d"]));
    let blockers = result["blockers"].as_array().expect("blockers");
    assert_eq!(blockers.len(), 1, "{blockers:?}");
    let blocker = blockers[0].as_str().unwrap_or_default();
    assert!(
        blocker.contains("html>body>canvas:nth-of-type(1)"),
        "{blocker}"
    );
    assert!(blocker.contains("could not be read"), "{blocker}");
}

/// The arm that decides the shape of the fix. A blank export from a surface that still
/// answers a 2D context is genuine absence, and it has to stay silent: every undrawn canvas
/// on every real page takes this branch, so recording it would bury the entries that mean
/// something. Blankness alone therefore cannot decide — only the context type can.
#[test]
fn leaves_a_surface_holding_a_two_dimensional_context_silent_when_nothing_was_drawn() {
    let result = read(&probed("blank(200x120)", "{}"));
    assert_eq!(result["attributes"], json!({}));
    assert_eq!(result["assets"], json!({}));
    assert_eq!(result["blockers"], json!([]));
}

/// Asking a canvas for a context *creates* one when it had none, so the probe is a mutation
/// of the page under capture. It is confined to surfaces already about to be reported as
/// empty: a surface that exported content is never probed at all.
#[test]
fn does_not_probe_the_context_of_a_surface_that_exported_its_content() {
    let result = read(&probed("data:image/png;base64,PAINTED", "null"));
    assert_eq!(result["probes"], json!([]));
    assert_eq!(
        result["attributes"],
        json!({ super::ATTRIBUTE: "recreate-surface:html>body>canvas:nth-of-type(1)" })
    );
    assert_eq!(result["blockers"], json!([]));
}
