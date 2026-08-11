use super::*;

/// The element names the gate admits, one per compound selector. Read as parsed names
/// rather than as substrings, because `imagesrcset` contains `image` and a substring test
/// would report the SVG raster as admitted while nothing matched it.
fn gated_elements() -> Vec<&'static str> {
    ASSET_SELECTOR
        .split(',')
        .map(|compound| compound.split('[').next().unwrap_or_default().trim())
        .collect()
}

/// Every reference whose fetch destination the platform governs with `img-src` or
/// `media-src` is painted into *this* document, so the artifact must contain its bytes.
/// The gate had enumerated media tags instead, which silently excluded `<input type=image>`
/// — a replaced element whose destination is `image`, exactly as `<img>`'s is.
///
/// Asserted name by name against Fetch's destination table rather than by matching one
/// selector string, because the point of the change is which references the rule reaches,
/// not how the predicate happens to be spelled.
#[test]
fn admits_every_element_whose_fetch_destination_is_painted_into_this_document() {
    let gated = gated_elements();
    for element in [
        "img", "image", "input", "video", "audio", "source", "track", "link",
    ] {
        assert!(
            gated.contains(&element),
            "an element painting a subresource is outside the gate: {element} in {gated:?}"
        );
    }
    assert!(
        ASSET_SELECTOR.contains("input[type=image"),
        "an unqualified input would collect a nothing-URL from an unrelated control"
    );
}

/// The complement, and the reason the rule is stated as a destination rather than as "it
/// loads something". `<object>` and `<embed>` are governed by `object-src` beside
/// `<iframe>`: they establish a nested browsing context, which is the document reference
/// the module excludes by name. Admitting them would cross the boundary, not widen it.
#[test]
fn excludes_the_references_that_load_another_document() {
    let gated = gated_elements();
    for element in ["iframe", "object", "embed", "a", "area", "frame"] {
        assert!(
            !gated.contains(&element),
            "a document reference reached the subresource gate: {element}"
        );
    }
}

/// SVG's `<image>` carries its URL in `href`, and content authored against SVG 1.1 carries
/// it in the XLink-namespaced `xlink:href`. Both are whole-value URLs, so both belong in
/// the attribute table beside `src`.
#[test]
fn names_the_url_attributes_an_svg_raster_actually_uses() {
    for name in ["href", "xlink:href"] {
        assert!(
            URL_ATTRIBUTES.contains(&name),
            "an SVG raster's URL attribute is unknown to the table: {name}"
        );
    }
    let assets = BTreeMap::from([(
        "https://example.test/plain.png".to_string(),
        "/assets/33.png".to_string(),
    )]);
    assert_eq!(
        rewrite("https://example.test/plain.png", &assets),
        "/assets/33.png"
    );
}

/// Namespacing must never reach a selector. `[href]` cannot match `xlink:href`, and the
/// syntax that could, `[xlink|href]`, needs an `@namespace` the Selectors API cannot
/// supply — so `[xlink:href]` is not a valid selector and *throws*. Both gates run this one
/// string over the whole document, so that SyntaxError would abort collection entirely and
/// stop every asset on the page from downloading, which is far worse than the omission
/// being fixed here. A namespaced attribute is handled by the table, where it is compared
/// as a plain string.
#[test]
fn keeps_every_namespaced_name_out_of_the_element_gate() {
    for qualifier in ASSET_SELECTOR.split('[').skip(1) {
        let qualifier = qualifier.split(']').next().unwrap_or_default();
        assert!(
            !qualifier.contains(':'),
            "a namespaced attribute selector would throw and abort collection: [{qualifier}]"
        );
    }
    assert_eq!(
        ASSET_SELECTOR.to_ascii_lowercase(),
        ASSET_SELECTOR,
        "a foreign element is matched case-sensitively by local name, so an upper-case \
         name in the gate would silently match nothing"
    );
}
