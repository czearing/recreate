use super::document;
use crate::model::PageState;
use std::collections::BTreeMap;

/// A head whose `<link rel="icon">` names an asset the capture already downloaded. The
/// capture resolves every gated URL against the document base, so the recorded value is
/// absolute; the asset map is keyed by that same absolute URL.
fn page() -> PageState {
    let node = |path: &str, parent: Option<&str>, tag: &str, attributes: serde_json::Value| {
        serde_json::json!({
            "path": path,
            "parent": parent,
            "tag": tag,
            "text": "",
            "attributes": attributes,
            "rect": {"x": 0.0, "y": 0.0, "width": 800.0, "height": 600.0},
            "style": {},
        })
    };
    serde_json::from_value(serde_json::json!({
        "url": "https://example.test/",
        "title": "Icon",
        "nodes": [
            node("html", None, "html", serde_json::json!({})),
            node("html>head:nth-of-type(1)", Some("html"), "head", serde_json::json!({})),
            node(
                "html>head:nth-of-type(1)>link:nth-of-type(1)",
                Some("html>head:nth-of-type(1)"),
                "link",
                serde_json::json!({"rel": "icon", "href": "https://example.test/icon.png"}),
            ),
            node("html>body:nth-of-type(1)", Some("html"), "body", serde_json::json!({})),
        ],
        "animations": [],
        "css_rules": [],
        "asset_urls": [],
    }))
    .unwrap()
}

fn assets() -> BTreeMap<String, String> {
    BTreeMap::from([(
        "https://example.test/icon.png".to_string(),
        "/assets/abcdef0123456789abcd.png".to_string(),
    )])
}

/// The shell is the second emitter of captured attributes, and a reference it writes is a
/// reference the artifact advertises. Downloading an icon's bytes while writing the
/// capture-time absolute URL beside them is worse than not collecting it at all: the file
/// exists, the page still requests the source origin, and a file-count check reports
/// success. Localisation therefore has to be the same call the JSX emitter makes, not a
/// second reading of the same rule.
#[test]
fn repoints_a_head_reference_at_the_downloaded_copy() {
    let html = document::render(Some(&page()), "", &BTreeMap::new(), &assets());
    assert!(
        html.contains("href=\"/assets/abcdef0123456789abcd.png\""),
        "document was {html}"
    );
    assert!(
        !html.contains("https://example.test/icon.png"),
        "the capture-time origin survived into the shell: {html}"
    );
}

/// A head reference the collector never admitted has no local copy, so the emitter has
/// nothing to substitute and must leave the value byte-identical rather than guess.
#[test]
fn leaves_a_reference_with_no_local_copy_untouched() {
    let html = document::render(Some(&page()), "", &BTreeMap::new(), &BTreeMap::new());
    assert!(
        html.contains("href=\"https://example.test/icon.png\""),
        "document was {html}"
    );
}
