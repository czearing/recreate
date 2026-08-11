use super::*;
use std::collections::BTreeMap;

const ORIGIN: &str = "http://127.0.0.1:5173/";

fn assets() -> BTreeMap<String, String> {
    ["tiny", "small", "large", "plain"]
        .iter()
        .enumerate()
        .map(|(index, name)| {
            (
                format!("{ORIGIN}{name}.png"),
                format!("/assets/{index}{index}.png"),
            )
        })
        .collect()
}

/// Every candidate in a descriptor list names an asset, so every candidate must be
/// localised. Rewriting only the whole value leaves the attribute the browser actually
/// obeys pointing at the capture origin.
#[test]
fn localises_every_candidate_in_a_descriptor_list() {
    let value = format!("{ORIGIN}tiny.png 50w, {ORIGIN}small.png 400w, {ORIGIN}large.png 1200w");
    assert_eq!(
        rewrite(&value, &assets()),
        "/assets/00.png 50w, /assets/11.png 400w, /assets/22.png 1200w"
    );
}

/// The descriptors are the reader's only record of which candidate is which. A rewriter
/// that localises the URLs but drops or reorders the descriptors destroys the attribute.
#[test]
fn keeps_descriptors_and_authored_spacing_beside_the_localised_urls() {
    let value = format!("{ORIGIN}small.png   2x,{ORIGIN}large.png");
    assert_eq!(
        rewrite(&value, &assets()),
        "/assets/11.png   2x,/assets/22.png"
    );
}

/// `sizes` shares the comma-separated shape of `srcset` but carries lengths and media
/// conditions, never URLs. Nothing in it can be an asset key, so it must come through
/// byte-identical rather than be tokenised into something new.
#[test]
fn leaves_a_value_that_holds_no_urls_untouched() {
    let sizes = "(max-width: 600px) 480px, (max-width: 900px) 800px, 100vw";
    assert_eq!(rewrite(sizes, &assets()), sizes);
}

/// A comma only ends a candidate where it trails one. Inside a URL it is ordinary data,
/// and splitting on it would shred every `data:` URL and every query string that uses it.
#[test]
fn treats_a_comma_inside_a_url_as_part_of_the_url() {
    let mut assets = assets();
    let comma_url = format!("{ORIGIN}chart.png?series=red,blue");
    assets.insert(comma_url.clone(), "/assets/99.png".into());
    let value = format!("{comma_url} 1x, {ORIGIN}large.png 2x");
    assert_eq!(
        rewrite(&value, &assets),
        "/assets/99.png 1x, /assets/22.png 2x"
    );
}

/// A single-URL attribute such as `src` is one token, so the same rule must still localise
/// it. This is the path that already worked and must keep working.
#[test]
fn localises_a_lone_url() {
    assert_eq!(
        rewrite(&format!("{ORIGIN}plain.png"), &assets()),
        "/assets/33.png"
    );
}

/// A candidate that was never downloaded has no key, so it must survive rather than be
/// replaced by a neighbour's path. Token-wise equality is what makes a shorter asset URL
/// unable to shadow a longer one that it prefixes.
#[test]
fn cannot_let_one_asset_url_shadow_another_it_prefixes() {
    let mut assets = BTreeMap::new();
    assets.insert(format!("{ORIGIN}a.png"), "/assets/aa.png".into());
    let value = format!("{ORIGIN}a.png 1x, {ORIGIN}a.png.bak 2x");
    assert_eq!(
        rewrite(&value, &assets),
        format!("/assets/aa.png 1x, {ORIGIN}a.png.bak 2x")
    );
}

/// The in-page rendering and the emit-side rendering must agree on which attributes carry
/// URLs, because the collector downloads what the rewriter will later look up. A name in
/// one list and not the other is exactly the shape of the defect this module exists to end.
#[test]
fn the_page_rendering_names_every_attribute_the_selector_can_carry() {
    let source = js_source();
    for name in URL_ATTRIBUTES.iter().chain(CANDIDATE_ATTRIBUTES.iter()) {
        assert!(
            source.contains(&format!("'{name}'")),
            "the page rendering never mentions {name}"
        );
    }
    assert!(source.contains(ASSET_SELECTOR));
}

/// The page rendering is spliced into a script the browser evaluates, where a syntax error
/// surfaces as a whole capture failing for an unrelated-looking reason. `node --check`
/// parses without executing, so the error is named here instead. A missing parser fails
/// rather than skips: a skipped check and a passing one read identically.
#[test]
fn the_page_rendering_parses_as_javascript() {
    let path = std::env::temp_dir().join("recreate_asset_attributes_check.js");
    std::fs::write(&path, js_source()).expect("cannot stage the rendered source");
    let output = std::process::Command::new("node")
        .arg("--check")
        .arg(&path)
        .output()
        .expect("cannot run `node --check`");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "the rendered page source did not parse:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
