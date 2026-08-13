//! Which authored conditions travel into the generated stylesheet, and which must not.

use super::{emitted, node};
/// The container query, which is the case a viewport sweep can never reach. Its condition
/// resolves against the used inline-size of the nearest ancestor with `container-type`, so
/// two instances of one component sit on opposite branches in the same paint and no viewport
/// band separates them. The prelude travels verbatim — name, `style()` query and all — rather
/// than being taken apart, so this stage needs no grammar for a syntax it does not own.
#[test]
fn carries_a_container_condition_the_way_it_carries_a_media_one() {
    for prelude in [
        "@container cardwrap (min-width: 500px)",
        "@container (min-width: 500px)",
        "@container style(--mode: wide)",
    ] {
        let rules = emitted(
            &node("card"),
            &[format!("{prelude} {{ .card {{ background: #3b5bdb; }} }}")],
        );
        assert_eq!(rules.len(), 1, "{prelude}: {rules:?}");
        assert!(
            rules[0].starts_with(prelude),
            "{prelude} lost its condition, publishing an unconditional rule: {rules:?}"
        );
        assert!(
            rules[0].contains(".generated") && rules[0].contains("background: #3b5bdb"),
            "{prelude} lost the declaration or its remapped class: {rules:?}"
        );
    }
}

/// The limit, so widening reach does not become "carry every at-rule". A feature query has
/// one answer for the run and the artifact does not reproduce the engine; `@scope` and
/// `@layer` preludes name authored selectors and cascade positions that do not exist in the
/// generated document. Re-emitting any of them references a name the output cannot resolve.
#[test]
fn carries_no_condition_the_generated_document_cannot_answer() {
    for prelude in [
        "@supports (display: grid)",
        "@scope (.card)",
        "@layer theme",
        "@media-hypothetical (min-width: 1px)",
    ] {
        let rules = emitted(
            &node("card"),
            &[format!("{prelude} {{ .card {{ color: red; }} }}")],
        );
        assert!(rules.is_empty(), "{prelude} was re-emitted: {rules:?}");
    }
}
