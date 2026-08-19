//! What the capture's one selector reader answers, asked of the shipped source under Node.
//!
//! Every question here is one a stage used to answer with `split(',')` or a regex. The
//! inputs are the shapes a shipped stylesheet actually authors — a comma inside a functional
//! pseudo-class, a comma inside a quoted attribute value, a state nested inside a forgiving
//! list, a state inside `:has()` — because those are the ones where a fragment cut out of the
//! middle of a selector is still a selector and so fails silently.

use serde_json::Value;

/// Evaluates `expression` against the shipped reader and returns what it produced.
fn ask(expression: &str) -> Value {
    let script = format!(
        "{}\nconsole.log(JSON.stringify({expression}));",
        super::SOURCE
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scan.js");
    std::fs::write(&path, script).unwrap();
    let output = std::process::Command::new("node")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "reader failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn members(selectors: &str) -> Vec<String> {
    serde_json::from_value(ask(&format!("selectorMembers({})", quoted(selectors)))).unwrap()
}

fn resting(selector: &str) -> String {
    serde_json::from_value(ask(&format!("restingSelector({})", quoted(selector)))).unwrap()
}

fn relation(selector: &str) -> Value {
    ask(&format!("stateRelation({})", quoted(selector)))
}

fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap()
}

/// The filed defect, at the level it happens. A list is separated by top-level commas only,
/// and every other comma belongs to the construct that encloses it.
#[test]
fn a_list_is_divided_only_where_the_grammar_divides_it() {
    for (selectors, expected) in [
        (
            ".root:where(:focus-visible,[data-activedescendant-focusvisible])",
            vec![".root:where(:focus-visible,[data-activedescendant-focusvisible])"],
        ),
        (
            ".root:has(.input:not([data-disabled],[aria-invalid=true]):focus-visible)",
            vec![".root:has(.input:not([data-disabled],[aria-invalid=true]):focus-visible)"],
        ),
        // The comma is string content here, not grammar, so nothing divides at it either.
        (r#"[title="a,b"]"#, vec![r#"[title="a,b"]"#]),
        // A list that really is one still divides, and an empty member is not a member.
        (".a, .b ,, .c", vec![".a", ".b", ".c"]),
        (
            ".a:is(.b, .c), .d:not(.e, .f)",
            vec![".a:is(.b, .c)", ".d:not(.e, .f)"],
        ),
    ] {
        assert_eq!(members(selectors), expected, "dividing {selectors}");
    }
}

/// A selector reduced to what it matches while the page rests. The nested cases are the ones
/// that decide the defect: deleting the pseudo-class alone leaves `:where(,[x])`, which does
/// not parse, and tidying that to `:where([x])` is valid and matches a different population.
#[test]
fn a_reduced_selector_names_the_element_the_state_will_land_on() {
    for (selector, expected) in [
        (".plainInput:focus-visible", ".plainInput"),
        (".teamRow:hover .teamBadge", ".teamRow .teamBadge"),
        (
            ".root:where(:focus-visible,[data-activedescendant-focusvisible])",
            ".root",
        ),
        // A branch that survives its state keeps everything the state was not.
        (".field:is(.large:hover, .small)", ".field:is(.large)"),
        // A state inside `:has()` is reduced within the relation, which still constrains the
        // subject: at rest the card is one that contains the button.
        (".card:has(.cardBtn:hover)", ".card:has(.cardBtn)"),
        // A list saying nothing about state is authored text and must arrive unaltered.
        (
            ".root:where(:not(.disabled,.readOnly))",
            ".root:where(:not(.disabled,.readOnly))",
        ),
        // A colon inside a quoted value is data, and reducing must not read it as grammar.
        (
            r#"[data-when="09:hover"]:hover"#,
            r#"[data-when="09:hover"]"#,
        ),
        // `:focus` is a prefix of `:focus-visible`, so a reader that stops at word characters
        // leaves `-visible` behind and produces a selector that matches nothing.
        (".a:focus-visible", ".a"),
        (".a:focus-within .b", ".a .b"),
    ] {
        assert_eq!(resting(selector), expected, "reducing {selector}");
    }
}

/// A pseudo-element takes an argument that may itself hold a comma, so where the name ends is
/// the same balanced question as everything else, and the subject is what precedes it.
#[test]
fn a_generated_box_is_separated_from_the_element_that_originates_it() {
    let box_of = |selector: &str| ask(&format!("generatedBoxOf({})", quoted(selector)));
    assert_eq!(
        box_of(".root:where(:not(.a,.b))::after")["suffix"],
        "::after"
    );
    assert_eq!(
        box_of(".root:where(:not(.a,.b))::after")["subject"],
        ".root:where(:not(.a,.b))"
    );
    assert_eq!(box_of(".host::part(a, b)")["suffix"], "::part");
    assert_eq!(box_of(".host::part(a, b)")["subject"], ".host");
    // A pseudo-element named inside a functional pseudo-class is not the member's own box.
    assert!(box_of(".a:not(::before)").is_null());
    assert_eq!(
        ask("withoutGeneratedBoxes('.host::part(a, b):hover')"),
        ".host:hover"
    );
}

/// Which of the two relations CSS can express this selector uses, and which element each side
/// of it names. The queries are handed to the engine; nothing downstream reads the text again.
#[test]
fn a_state_inside_a_relational_pseudo_class_is_held_by_something_the_subject_contains() {
    let inside =
        relation(".root:has(.input:not([data-disabled],[aria-invalid=true]):focus-visible)");
    assert_eq!(inside["contained"], true);
    assert_eq!(inside["query"], ".root");
    assert_eq!(
        inside["holder"],
        ".input:not([data-disabled],[aria-invalid=true])"
    );
    assert_eq!(inside["states"], serde_json::json!([":focus-visible"]));

    let above = relation(".teamRow:hover .teamBadge");
    assert_eq!(above["contained"], false);
    assert_eq!(above["query"], ".teamRow .teamBadge");
    assert_eq!(above["holder"], ".teamRow");

    // A state on the subject itself resolves to the subject, which is how the holder search
    // returns the element it started from and nothing is scoped.
    let self_held = relation(".plainInput:focus-visible");
    assert_eq!(self_held["contained"], false);
    assert_eq!(self_held["query"], ".plainInput");
    assert_eq!(self_held["holder"], ".plainInput");

    // Nested, and still relational: the holder is the branch of the inner list carrying the
    // state, reduced, and the subject is everything the relation was attached to.
    let nested = relation(".card:has(.btn:where(:hover,[data-pressed]))");
    assert_eq!(nested["contained"], true);
    assert_eq!(nested["query"], ".card");
    assert_eq!(nested["holder"], ".btn");
}
