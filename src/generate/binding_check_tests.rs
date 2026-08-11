use super::unbound;

/// The checker has to discriminate, or the project-wide test below proves nothing. A flat
/// "collect every declared name" pass reports this file clean, because `width` *is* declared
/// twice — as a parameter of two other functions, neither of which encloses the reference.
/// That is exactly the shape the defect hid behind in the real `App.jsx`, and it is why the
/// verdict must come from a scope resolver rather than from a search.
#[test]
fn does_not_accept_a_binding_from_a_scope_that_does_not_enclose_the_reference() {
    let source = concat!(
        "const selectViewport=(width,widths)=>widths.indexOf(width);\n",
        "const subscribe=notify=>[390,320].map(width=>notify(width));\n",
        "export const clamp=()=>width<=390?5:0;\n",
    );
    assert_eq!(unbound(source, "decoy.jsx"), ["width"]);
}

/// The other half of discrimination. A name really bound by an enclosing scope, by an
/// import, or by the runtime's own global list must not be reported — a checker that flags
/// everything would also pass the test above while blocking every capture.
#[test]
fn accepts_a_name_an_enclosing_scope_an_import_or_the_runtime_supplies() {
    let source = concat!(
        "import React from 'react';\n",
        "const widths=[390];\n",
        "export const clamp=viewport=>widths[viewport]<=390&&document.body?<React.Fragment/>:0;\n",
    );
    assert!(unbound(source, "bound.jsx").is_empty());
}

/// A name the browser supplies but this runtime never uses stays unbound. `globals.browser`
/// declares `top`, `name`, `status`, `length`, `self` and `parent`, so adopting it wholesale
/// would let a fragment meaning a local by one of those names resolve against the window
/// instead. The allow-list is the runtime's own vocabulary for exactly that reason.
#[test]
fn does_not_admit_a_browser_global_the_runtime_never_uses() {
    for name in ["top", "name", "status", "length", "self", "parent"] {
        let source = format!("export const read=()=>{name}<=390;\n");
        assert_eq!(unbound(&source, "shadowable.jsx"), [name], "{name}");
    }
}
