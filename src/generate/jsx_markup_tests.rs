use super::{attribute_values, to_xml};
use crate::generate::jsx_attr_names::jsx_attribute;

/// The relocation path end to end: names as the capture recorded them, through the JSX
/// spelling, into a standalone document with no renderer downstream to repair them.
///
/// The two stages used to disagree about their domain — the outbound conversion was a rule
/// that answered for every name, the inbound one a search over a fixed list — so a name the
/// list did not hold arrived here camel-cased and stayed that way. XML matches attribute
/// names case-sensitively, so `colorRendering` is not `color-rendering`; it is an unknown
/// attribute. Nothing was dropped and nothing errored, and the rendered page was correct
/// either way because React repairs the same name at render time, which is why this has to
/// be asserted on the emitted document rather than on the page.
///
/// Whole-tag equality rather than a search for one name, so the check also fails if a fix
/// over-corrects and hyphenates the camelCase spellings SVG owns.
#[test]
fn a_relocated_document_carries_the_names_the_capture_recorded() {
    let captured = [
        // SVG's own camelCase spelling, which no inverse may touch.
        ("viewBox", "0 0 100 100"),
        ("aria-hidden", "true"),
        // In the list, so it was always restored: the control.
        ("stroke-width", "6"),
        // Real multi-word presentation attributes the list does not name.
        ("color-rendering", "optimizeSpeed"),
        ("enable-background", "new 0 0 100 100"),
        // A name no specification will add, which is what proves the rule needs no list.
        ("author-invented", "kept"),
    ];
    let jsx: String = captured
        .iter()
        .map(|(name, value)| format!(" {}={{\"{value}\"}}", jsx_attribute(name)))
        .collect();
    let expected: String = captured
        .iter()
        .map(|(name, value)| format!(" {name}=\"{value}\""))
        .collect();
    assert_eq!(
        to_xml(&format!("<circle{jsx} />")),
        format!("<circle{expected} />")
    );
}

/// The defect this module exists to close: `clipPath` is both an SVG element name and a
/// React attribute name, so a rename folded over the flat string rewrote the element as
/// well as the attribute. Both roles must appear correctly in one document, because
/// repairing either alone leaves the clip lost.
#[test]
fn renames_an_attribute_without_renaming_the_element_of_the_same_name() {
    let xml = to_xml(concat!(
        r#"<clipPath id={"halfClip"}><rect width={"60"} /></clipPath>"#,
        r#"<rect clipPath={"url(#halfClip)"} />"#,
    ));
    assert!(xml.contains("<clipPath id=\"halfClip\">"), "{xml}");
    assert!(xml.contains("</clipPath>"), "{xml}");
    assert!(!xml.contains("clip-path id="), "{xml}");
    assert!(xml.contains("clip-path=\"url(#halfClip)\""), "{xml}");
}

/// A half-anchored rename that repairs the opening tag but not the closing one produces an
/// unbalanced document, which fails XML parsing outright rather than merely losing a clip.
#[test]
fn keeps_every_tag_pair_balanced() {
    let xml = to_xml(r#"<defs><clipPath id={"c"}><path /></clipPath><textPath /></defs>"#);
    let opened = xml.matches("<clipPath").count();
    let closed = xml.matches("</clipPath>").count();
    assert_eq!(opened, closed, "unbalanced: {xml}");
    assert!(xml.contains("<textPath"), "{xml}");
}

/// A rename must not reach into attribute values, where the same letters are page content
/// rather than a name the converter owns.
#[test]
fn leaves_the_same_letters_alone_inside_attribute_values_and_text() {
    let xml = to_xml(r#"<g className={"clipPathWrapper"} id={"clipPath1"}>{"clipPath"}</g>"#);
    assert!(xml.contains("class=\"clipPathWrapper\""), "{xml}");
    assert!(xml.contains("id=\"clipPath1\""), "{xml}");
    assert!(xml.contains(">clipPath</g>"), "{xml}");
}

/// SVG spells a large family of attributes in camelCase itself. Hyphenating them by rule
/// would corrupt every one, so the converter renames only names whose hyphenated spelling
/// is a CSS property and passes everything else through unchanged.
#[test]
fn preserves_attributes_svg_itself_spells_in_camel_case() {
    let xml = to_xml(concat!(
        r#"<svg viewBox={"0 0 1 1"} preserveAspectRatio={"none"}>"#,
        r#"<radialGradient gradientUnits={"userSpaceOnUse"} spreadMethod={"pad"} />"#,
        r#"<feGaussianBlur stdDeviation={"2"} /><path pathLength={"1"} /></svg>"#,
    ));
    for kept in [
        "viewBox=",
        "preserveAspectRatio=",
        "gradientUnits=",
        "spreadMethod=",
        "stdDeviation=",
        "pathLength=",
        "<radialGradient",
        "<feGaussianBlur",
    ] {
        assert!(xml.contains(kept), "{kept} was rewritten: {xml}");
    }
}

/// The nine-name table this replaced covered a fraction of the hyphenated family, so any
/// page using the rest emitted camelCase names no SVG renderer knows.
#[test]
fn renames_the_whole_hyphenated_family_not_only_the_names_seen_so_far() {
    let xml = to_xml(concat!(
        r#"<text strokeDasharray={"4 2"} strokeMiterlimit={"2"} fontFamily={"serif"}"#,
        r#" textAnchor={"middle"} dominantBaseline={"middle"} markerEnd={"url(#a)"}"#,
        r#" paintOrder={"stroke"} vectorEffect={"non-scaling-stroke"} floodColor={"red"} />"#,
    ));
    for renamed in [
        "stroke-dasharray=",
        "stroke-miterlimit=",
        "font-family=",
        "text-anchor=",
        "dominant-baseline=",
        "marker-end=",
        "paint-order=",
        "vector-effect=",
        "flood-color=",
    ] {
        assert!(xml.contains(renamed), "{renamed} missing: {xml}");
    }
}

#[test]
fn unwraps_serialized_values_text_and_boolean_props() {
    let xml = to_xml(r#"<g><text x={"1"}>{"a > b"}</text><input disabled={true} /></g>"#);
    assert!(xml.contains("x=\"1\""), "{xml}");
    assert!(xml.contains(">a &gt; b<"), "{xml}");
    assert!(xml.contains("disabled=\"true\""), "{xml}");
    assert!(!xml.contains('{'), "brace expression survived: {xml}");
}

/// Values are serialized by `serde_json`, so a quote inside page text is escaped rather
/// than terminating the literal. Splitting on the quote character would invert the parity
/// of every value after it.
#[test]
fn reads_values_written_after_an_escaped_quote() {
    let source = r#"<g title={"say \"hi"} className={"r0000000001"}><path d={"M0 0"} /></g>"#;
    assert_eq!(attribute_values(source, "className"), vec!["r0000000001"]);
    assert!(to_xml(source).contains("d=\"M0 0\""));
}

#[test]
fn reads_every_occurrence_of_an_attribute_in_document_order() {
    let source = r#"<g className={"a"}><g className={"b c"} /></g>"#;
    assert_eq!(attribute_values(source, "className"), vec!["a", "b c"]);
}

/// `className` in an attribute value or in text is not a binding, so a scanner that finds
/// attributes by substring would invent classes the document never carried.
#[test]
fn does_not_mistake_the_attribute_name_appearing_as_content_for_a_binding() {
    let source = r#"<g id={"className=x"}>{"className={\"ghost\"}"}</g>"#;
    assert!(attribute_values(source, "className").is_empty());
}

#[test]
fn preserves_self_closing_elements() {
    let xml = to_xml(r#"<g><path d={"M0 0"} /></g>"#);
    assert!(xml.contains("<path d=\"M0 0\" />"), "{xml}");
}

#[test]
fn converts_jsx_svg_to_xml() {
    let xml =
        to_xml(r#"<svg className={"icon"} viewBox={"0 0 1 1"}><path strokeWidth={"1"} /></svg>"#);
    assert!(xml.contains("class=\"icon\""), "{xml}");
    assert!(xml.contains("stroke-width=\"1\""), "{xml}");
    assert!(xml.contains("viewBox=\"0 0 1 1\""), "{xml}");
}
