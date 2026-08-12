//! The single owner of "no injected script reaches an element's inline style through the
//! instance".
//!
//! Every script here is evaluated inside a page the tool does not control, so each property
//! it names on an element is a name that page may already have taken. A class field —
//! `class B extends HTMLElement { style = v }` — is installed with `[[DefineOwnProperty]]`,
//! which ignores the prototype chain and shadows the `style` accessor with a plain value.
//! The next `.style.setProperty(...)` then throws, and because these scripts are evaluated
//! as one expression whose rejection is the capture's result, a throw anywhere in them ends
//! the whole run with no artifact at all.
//!
//! Guarding the call would be worse than the abort: the run would exit 0 while the shadowed
//! elements were silently absent from the output. The repair is to stop reaching that way.
//! Inline style is reachable through `getAttribute`/`setAttribute`, which these scripts
//! already use to save and restore it, so the shadowable path was a second spelling of one
//! the scripts owned rather than a capability. Deleting it leaves one access path, and this
//! is the check that keeps it at one.
//!
//! The invariant is stated over the parsed program rather than the text because the text
//! form is what let the defect ship: the previous test asserted the *presence* of the
//! broken call, so it stayed green through a runtime `TypeError`. A chained read is
//! `<expr>.style.<name>` — reaching the declaration block and then through it. Reading
//! `.style` as a value is not that, and the captured-node records these scripts build carry
//! a plain `style` object of their own that this must not confuse for an element.

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, StaticMemberExpression};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Every script this tool evaluates inside the captured page.
const INJECTED: &[(&str, &str)] = &[
    ("asset_attributes.js", include_str!("asset_attributes.js")),
    (
        "interaction_script.js",
        include_str!("interaction_script.js"),
    ),
    (
        "lifecycle_mutations.js",
        include_str!("lifecycle_mutations.js"),
    ),
    ("lifecycle_script.js", include_str!("lifecycle_script.js")),
    ("page_capture.js", include_str!("page_capture.js")),
    (
        "style_baseline_script.js",
        include_str!("style_baseline_script.js"),
    ),
    ("surface_content.js", include_str!("surface_content.js")),
    ("asset_script.rs", crate::asset_script::DOWNLOADS),
    (
        "attribute_sequence_script.rs",
        crate::attribute_sequence_script::TEMPLATE,
    ),
    (
        "lifecycle_scheduled_script.rs",
        crate::lifecycle_scheduled_script::SOURCE,
    ),
    (
        "lifecycle_settle_script.rs",
        crate::lifecycle_settle_script::SOURCE,
    ),
    (
        "rule_activation_script.rs",
        crate::rule_activation_script::SOURCE,
    ),
];

/// The property read from `<expr>.style` at every site that reads through it. Empty is the
/// only passing result.
fn chained_style_reads(source: &str, filename: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "{filename} does not parse: {:?}",
        parsed.diagnostics
    );
    let mut reaches = Reaches {
        filename,
        found: Vec::new(),
    };
    reaches.visit_program(&parsed.program);
    reaches.found
}

/// Records every `<expr>.style.<name>` the program contains, wherever it is nested.
struct Reaches<'a> {
    filename: &'a str,
    found: Vec<String>,
}

impl<'a> Visit<'a> for Reaches<'_> {
    fn visit_static_member_expression(&mut self, member: &StaticMemberExpression<'a>) {
        if reads_style(&member.object) {
            let (filename, property) = (self.filename, member.property.name);
            self.found.push(format!("{filename}: .style.{property}"));
        }
        walk::walk_static_member_expression(self, member);
    }
}

/// Whether `expression` is itself a read of a `style` property, whatever it is read from.
fn reads_style(expression: &Expression) -> bool {
    match expression {
        Expression::StaticMemberExpression(inner) => inner.property.name == "style",
        Expression::ChainExpression(chain) => chain
            .expression
            .as_member_expression()
            .and_then(|member| member.static_property_name())
            .is_some_and(|name| name == "style"),
        _ => false,
    }
}

#[test]
fn no_injected_script_reaches_inline_style_through_the_instance() {
    let reaches = INJECTED
        .iter()
        .flat_map(|(filename, source)| chained_style_reads(source, filename))
        .collect::<Vec<_>>();
    assert!(
        reaches.is_empty(),
        "a page can shadow `style` with an own property, so these reads abort the capture: {reaches:#?}"
    );
}

#[test]
fn the_check_recognises_the_construct_it_exists_to_forbid() {
    assert_eq!(
        chained_style_reads(
            "el.style.setProperty('all', 'revert', 'important');",
            "f.js"
        ),
        ["f.js: .style.setProperty"]
    );
    assert_eq!(
        chained_style_reads("el?.style.cssText;", "f.js"),
        ["f.js: .style.cssText"]
    );
    assert!(chained_style_reads("Object.values(node.style || {});", "f.js").is_empty());
    assert!(chained_style_reads("el.getAttribute('style').length;", "f.js").is_empty());
}
