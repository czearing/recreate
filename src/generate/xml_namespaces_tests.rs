use super::source_svg_assets::document;

/// The assertion the whole item rests on: an emitted asset is only useful if an XML parser
/// accepts it. A substring check describes the fix; this describes the consequence, and it
/// is the only form that fails for every future prefix rather than for the one named.
fn parse(xml: &str) {
    if let Err(error) = roxmltree::Document::parse(xml) {
        panic!("asset is not namespace-well-formed: {error}\n{xml}");
    }
}

fn root(xml: &str) -> String {
    xml[..xml.find('>').expect("no root start tag")].into()
}

const USE: &str = concat!(
    r##"<svg viewBox={"0 0 48 48"}><defs><path id={"bar"} /></defs>"##,
    r##"<use xlink:href={"#bar"} /><use xlink:href={"#bar"} /></svg>"##,
);

#[test]
fn declares_the_prefix_its_own_markup_uses() {
    let xml = document(USE, "");
    assert!(
        root(&xml).contains(r#"xmlns:xlink="http://www.w3.org/1999/xlink""#),
        "xlink used but never bound: {}",
        root(&xml)
    );
    parse(&xml);
}

/// Relocation must not edit meaning. Rewriting `xlink:href` to SVG 2's unprefixed `href`
/// would satisfy the parser and change which consumers honour the link.
#[test]
fn binds_the_link_rather_than_rewriting_it() {
    let xml = document(USE, "");
    assert_eq!(xml.matches("xlink:href=").count(), 2, "{xml}");
}

/// The default namespace is the first case of the rule, not a sibling of it.
#[test]
fn still_declares_the_default_namespace() {
    let xml = document(r#"<svg viewBox={"0 0 1 1"} />"#, "");
    assert!(
        root(&xml).contains(r#"xmlns="http://www.w3.org/2000/svg""#),
        "{}",
        root(&xml)
    );
    parse(&xml);
}

/// Other members of the same prefix. A fix keyed to `xlink:href` would miss these.
#[test]
fn declares_a_prefix_reached_through_any_of_its_attributes() {
    let xml = document(
        r#"<svg><a xlink:title={"t"} xlink:show={"new"} xlink:actuate={"onRequest"} /></svg>"#,
        "",
    );
    assert!(root(&xml).contains("xmlns:xlink="), "{}", root(&xml));
    parse(&xml);
}

/// The rule is about names, so a prefixed element name binds exactly as an attribute does.
#[test]
fn declares_a_prefix_carried_by_an_element_name() {
    let xml = document(r#"<svg><ink:annotation id={"a"} /></svg>"#, "");
    assert!(root(&xml).contains("xmlns:ink="), "{}", root(&xml));
    parse(&xml);
}

/// `xml` is bound implicitly by the XML Namespaces specification. Declaring it is itself an
/// error, so the sweep must exempt it while still leaving the name it qualifies in place.
#[test]
fn never_declares_the_predefined_xml_prefix() {
    let xml = document(r#"<svg><text xml:space={"preserve"}>a b</text></svg>"#, "");
    assert!(
        !xml.contains("xmlns:xml="),
        "declared a reserved prefix: {xml}"
    );
    assert!(xml.contains(r#"xml:space="preserve""#), "{xml}");
    parse(&xml);
}

/// A document that already binds a prefix must be left alone: a duplicate attribute on one
/// start tag is a well-formedness violation, so the careless repair is as fatal as the bug.
#[test]
fn does_not_duplicate_a_declaration_the_document_already_carries() {
    let xml = document(
        concat!(
            r##"<svg xmlns={"http://www.w3.org/2000/svg"} "##,
            r##"xmlns:xlink={"http://www.w3.org/1999/xlink"}>"##,
            r##"<use xlink:href={"#bar"} /></svg>"##,
        ),
        "",
    );
    assert_eq!(root(&xml).matches("xmlns=").count(), 1, "{}", root(&xml));
    assert_eq!(
        root(&xml).matches("xmlns:xlink=").count(),
        1,
        "{}",
        root(&xml)
    );
    parse(&xml);
}

/// Declarations are scoped to the element that writes them, so a descendant's binding says
/// nothing about the root. Asking the whole serialized subtree whether a declaration is
/// present is the same scope error the attribute reader carried, one file away.
#[test]
fn declares_on_the_root_despite_a_descendant_that_declares_for_itself() {
    let xml = document(
        r#"<svg><foreignObject><div xmlns={"http://www.w3.org/1999/xhtml"} /></foreignObject></svg>"#,
        "",
    );
    assert!(
        root(&xml).contains(r#"xmlns="http://www.w3.org/2000/svg""#),
        "a descendant's declaration starved the root: {}",
        root(&xml)
    );
    parse(&xml);
}

/// A self-closing root has no content model, so an insertion after its first `>` lands
/// beside it and the asset becomes two top-level elements — as fatal as an unbound prefix,
/// and invisible to any check that only greps for the stylesheet.
#[test]
fn carries_the_stylesheet_inside_a_self_closing_root() {
    let xml = document(
        r#"<svg className={"r_a"} viewBox={"0 0 1 1"} />"#,
        ".r_a{fill:red;}",
    );
    assert!(xml.ends_with("</svg>"), "{xml}");
    let parsed = roxmltree::Document::parse(&xml).expect("not well-formed");
    let child = parsed
        .root_element()
        .first_element_child()
        .expect("no child");
    assert_eq!(child.tag_name().name(), "style", "{xml}");
    assert!(
        child.text().unwrap_or_default().contains("fill:red"),
        "{xml}"
    );
}

/// An unrecognised prefix is still a name the original carried. Dropping it edits the
/// document; binding it to an opaque namespace keeps the name and value intact and inert.
#[test]
fn binds_an_unrecognised_prefix_rather_than_dropping_the_name() {
    let xml = document(r#"<svg><path inkscape:label={"layer"} /></svg>"#, "");
    assert!(xml.contains(r#"inkscape:label="layer""#), "{xml}");
    parse(&xml);
}
