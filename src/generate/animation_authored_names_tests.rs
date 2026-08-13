use super::authored_names;
use std::collections::BTreeSet;

/// Every at-rule whose block holds other rules, so every wrapper a definition can legally
/// sit inside. Enumerated here rather than reused from `css::GROUPING_AT_RULES` on purpose:
/// a test that imports the list under test agrees with it by construction and would keep
/// passing if a name were dropped from it.
const WRAPPERS: &[&str] = &[
    "@media (min-width: 1px)",
    "@supports (rotate: 0deg)",
    "@container (min-width: 1px)",
    "@layer motion",
    "@scope (.card)",
    "@starting-style",
];

const DEFINITION: &str = "@keyframes spin{from{rotate:0deg;}to{rotate:360deg;}}";

fn names(css: &str) -> BTreeSet<String> {
    authored_names(css)
}

/// The invariant, stated as a relation rather than an example.
///
/// Wrapping a definition changes where it sits, never what it defines. Asserting that as a
/// relation between two outputs covers every wrapper and every definition kind, including
/// ones added later, and cannot be satisfied by special-casing the wrapper that was
/// reported — which an example-based assertion on `@supports` alone would permit.
///
/// This is the seam the earlier grouping-rule work did not close. `css::retain` learned to
/// descend, and three of the four readers of `css::global_rule` were routed through it. This
/// reader kept filtering a flat list, so it still answers as though nesting did not exist.
#[test]
fn a_wrapper_does_not_change_which_names_the_stylesheet_defines() {
    let bare = names(DEFINITION);
    assert_eq!(
        bare,
        BTreeSet::from(["spin".to_string()]),
        "an unwrapped definition must be reported"
    );
    for wrapper in WRAPPERS {
        let wrapped = format!("{wrapper}{{{DEFINITION}}}");
        assert_eq!(
            names(&wrapped),
            bare,
            "{wrapper} changed which names the stylesheet defines"
        );
    }
}

/// Nesting is not limited to one level and the orders are interchangeable, so a walk that
/// descends exactly once closes the reported case and leaves the rest open.
#[test]
fn a_definition_two_wrappers_deep_is_reported_in_either_order() {
    let expected = BTreeSet::from(["spin".to_string()]);
    for (outer, inner) in [
        ("@media (min-width: 1px)", "@layer motion"),
        ("@layer motion", "@media (min-width: 1px)"),
        ("@supports (rotate: 0deg)", "@media (min-width: 1px)"),
    ] {
        let css = format!("{outer}{{{inner}{{{DEFINITION}}}}}");
        assert_eq!(names(&css), expected, "{outer} around {inner}");
    }
}

/// The vendor-prefixed spelling is the same definition under another name, and a stylesheet
/// that carries both must not report the animation as undefined under either.
#[test]
fn a_prefixed_definition_is_reported_wrapped_or_not() {
    let prefixed = "@-webkit-keyframes spin{from{rotate:0deg;}}";
    assert_eq!(names(prefixed), BTreeSet::from(["spin".to_string()]));
    assert_eq!(
        names(&format!("@media (min-width: 1px){{{prefixed}}}")),
        BTreeSet::from(["spin".to_string()]),
    );
}

/// Widening reach must not become "report every at-rule". A grouping rule's own prelude
/// splits on whitespace into something that reads like an identifier — `(min-width:` for a
/// media query — so a walk that asks the wrapper instead of its members invents names that
/// no element can ever reference, and the sampler would then decline to rebuild an
/// animation that genuinely has no definition.
#[test]
fn a_wrapper_prelude_is_never_mistaken_for_a_definition() {
    let css = "@media (min-width: 1px){.card{color:red;}}";
    assert!(
        names(css).is_empty(),
        "read a name out of a wrapper that defines none: {:?}",
        names(css)
    );
    assert!(names("@font-face{font-family:Vorplish;}").is_empty());
    assert!(names(".card{animation-name:spin;}").is_empty());
}

/// A reference is not a definition. The whole point of the set is to separate names the
/// stylesheet defines from names it merely mentions, so a stylesheet that references a name
/// it never defines must report nothing and let the sampler rebuild it.
#[test]
fn a_reference_without_a_definition_is_not_reported() {
    let css = "@media (min-width: 1px){.card{animation:spin 4s linear infinite;}}";
    assert!(
        names(css).is_empty(),
        "a mention was counted as a definition, so the sampler will not rebuild it"
    );
}
