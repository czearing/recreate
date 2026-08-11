//! The single owner of "the generated project references no name it fails to bind".
//!
//! The app module is spliced together from fragments under `templates/`, and each fragment
//! is validated on its own by `node --check`. That guard is real but structurally blind
//! here: `node --check` parses and resolves nothing, and a fragment cannot be scope-checked
//! alone anyway, because its whole purpose is to close over bindings declared in the file it
//! is spliced into. In its own file every one of those names is free. So the invariant is
//! not a property of any fragment — it is a property of the *assembled* module, and this is
//! the only place that reads that artifact.
//!
//! The check errs toward silence rather than noise, which is the opposite of the predicate
//! that decides whether a fragment may be lifted into its own module. That one authorises a
//! refusal, so over-reporting costs one unshared fragment. This one fails the build, so a
//! false positive would block every capture — and a gate that blocks on findings nobody can
//! act on is removed wholesale, taking its true positives with it. Hence a real scope
//! resolver rather than a scan, and an allow-list of the globals this runtime actually uses
//! rather than the browser environment at large.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

/// The globals the generated runtime genuinely reaches for, and nothing else.
///
/// This list is measured rather than guessed: it is exactly the set of free names the
/// emitted project still carries once its own defect is removed. Naming them individually is
/// the point. The blanket browser environment also declares `top`, `name`, `status`,
/// `length`, `self` and `parent` — ordinary names for a local — so adopting it would let a
/// future fragment that means a local by one of those names resolve silently against the
/// window instead. Every entry here is either an ECMAScript or DOM constructor or a function
/// the runtime calls, none of which a captured page can shadow, so widening this list is a
/// visible edit rather than a default.
const RUNTIME_GLOBALS: [&str; 31] = [
    "Array",
    "CSSStyleSheet",
    "DOMMatrixReadOnly",
    "Event",
    "Map",
    "Math",
    "Number",
    "Object",
    "Set",
    "String",
    "TypeError",
    "WeakMap",
    "addEventListener",
    "cancelAnimationFrame",
    "clearTimeout",
    "document",
    "getComputedStyle",
    "globalThis",
    "location",
    "matchMedia",
    "performance",
    "queueMicrotask",
    "removeEventListener",
    "requestAnimationFrame",
    "scrollTo",
    "scrollX",
    "scrollY",
    "sessionStorage",
    "setTimeout",
    "undefined",
    "window",
];

/// Every name `source` references without binding it, an import supplying it, or this
/// runtime declaring it as a global. Empty is the only passing result.
pub(super) fn unbound(source: &str, filename: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::jsx();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "{filename} does not parse: {:?}",
        parsed.diagnostics
    );
    let semantic = SemanticBuilder::new().build(&parsed.program);
    let mut names = semantic
        .semantic
        .scoping()
        .root_unresolved_references()
        .keys()
        .map(|name| name.to_string())
        .filter(|name| !RUNTIME_GLOBALS.contains(&name.as_str()))
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
#[path = "binding_check_tests.rs"]
mod tests;
