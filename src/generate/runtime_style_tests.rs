//! The shipped style owner, run under Node against constructed-stylesheet doubles.
//!
//! Adoption is what decides whether a restored shadow subtree renders styled or bare, and it
//! is the one part of the repair that no artifact inspection can see: the emitted files are
//! identical either way, and the difference appears only when a root is opened at run time.

use crate::node_eval::evaluate;

const STYLE: &str = include_str!("../../runtime/style.mjs");

/// The module under a document double. `export` is stripped so the definitions land in the
/// same scope as the expression, which is what lets one file stand in for an import.
fn scene(body: &str) -> String {
    format!(
        "const document={{adoptedStyleSheets:[]}};\nglobalThis.CSSStyleSheet=class{{replaceSync(css){{this.css=css}}}};\n{}\nconst adopted=root=>root.adoptedStyleSheets.map(sheet=>sheet.css);\n{body}",
        STYLE.replace("export ", "")
    )
}

/// A shadow tree is a separate style scope, so a root opened after the document was styled
/// starts with nothing. Everything already registered has to follow it in, or the restored
/// subtree renders unstyled — which reads as a fidelity defect rather than as a crash.
#[test]
fn gives_a_newly_opened_root_every_sheet_the_document_already_had() {
    let scene = scene(
        "adoptRegisteredStyles(['a{}','b{}']);\nconst root={adoptedStyleSheets:[]};\nadoptInto(root);",
    );
    assert_eq!(
        evaluate(&scene, "[adopted(root), adopted(document)]"),
        serde_json::json!([["a{}", "b{}"], ["a{}", "b{}"]])
    );
}

/// The other order, which a cache keyed only on the document would drop: styles registered
/// while the page is running must reach every root already open, not just the document.
#[test]
fn carries_a_later_registration_into_every_root_already_open() {
    let scene = scene(
        "adoptRegisteredStyles(['a{}']);\nconst root={adoptedStyleSheets:[]};\nadoptInto(root);\nregisterStyle('b{}');",
    );
    assert_eq!(
        evaluate(&scene, "[adopted(root), adopted(document)]"),
        serde_json::json!([["a{}", "b{}"], ["a{}", "b{}"]])
    );
}

/// One sheet object serves every root that adopts it, so the same registration must not
/// construct a second one — and a root adopted twice must not accumulate duplicates.
#[test]
fn constructs_one_sheet_per_registration_however_many_roots_adopt_it() {
    let scene = scene(
        "adoptRegisteredStyles(['a{}']);\nconst root={adoptedStyleSheets:[]};\nadoptInto(root);\nadoptInto(root);\nregisterStyle('a{}');",
    );
    assert_eq!(
        evaluate(
            &scene,
            "[adopted(root), root.adoptedStyleSheets[0]===document.adoptedStyleSheets[0]]"
        ),
        serde_json::json!([["a{}"], true])
    );
}
