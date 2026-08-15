//! The single owner of "which URL-bearing values does this attribute contain?".
//!
//! An attribute that names an asset has to be recognised twice: once in the page, so the
//! bytes are downloaded, and once at emit time, so the reference is repointed at the local
//! copy. Those two sites had been written separately and were not co-extensive. The
//! collector read one URL per element — `currentSrc || src`, whichever single candidate
//! that browser picked at that viewport — while the emitter was handed `srcset`, which
//! names a whole set. Every candidate the capture browser did not choose was advertised by
//! the artifact and present nowhere in it, and a `<source>`, which has neither `src` nor
//! `currentSrc`, contributed no bytes at all.
//!
//! The two collectors had already drifted from each other as well: the interaction pass
//! scanned only `background-image`, the baseline pass scanned stylesheet text too.
//!
//! A selector states two things in one syntax — which nodes match, and which tree is
//! searched — and naming the first once said nothing about the second. The gate below was
//! read by a `matches` test on an element the walk already held, which inherits the walk's
//! reach into shadow roots, and by a `document.querySelectorAll`, which is confined to one
//! node tree. The two agreed on the light tree and disagreed everywhere else, so a shadow
//! `<img>` had its `src` resolved against the page base and its bytes never fetched: the
//! artifact shipped a live address on the capture rig's ephemeral port, dead after the run
//! and different on the next one. So collection is no longer a query at all. Resolving a
//! reference and recording it are one statement, made where the walk holds the element,
//! and reach stops being a decision this module makes: it is whatever traversal called in.
//!
//! So the table of URL-bearing attributes, the element selector, and the candidate-list
//! grammar are named once here and rendered twice, the way `blocking_overlay` renders its
//! predicate. The page rendering resolves each URL against the document base before
//! recording it, which is what lets the emit rendering stay an exact lookup: after capture
//! every URL is absolute and percent-encoded, so no URL contains whitespace, and a token
//! that is not a URL cannot equal an asset key. That is why the emit side needs no table.

use std::collections::BTreeMap;

/// Attributes whose entire value is one URL. `href` is here for the SVG raster and the
/// icon/preload `<link>`; content authored against SVG 1.1 spells the same attribute
/// `xlink:href`, in the XLink namespace. Namespacing is handled here rather than in the
/// gate below, where a `[xlink:href]` qualifier would not be a valid selector at all.
const URL_ATTRIBUTES: [&str; 4] = ["src", "poster", "href", "xlink:href"];
/// Attributes whose value is a list of candidates, each a URL followed by descriptors.
const CANDIDATE_ATTRIBUTES: [&str; 2] = ["srcset", "imagesrcset"];
/// The references whose bytes the artifact must contain, derived from Fetch's destination
/// table rather than collected as tags. A destination governed by `img-src` (`image`) or
/// `media-src` (`audio`, `video`, `track`) is painted into *this* document, so the artifact
/// is not self-contained without it. A destination governed by `child-src` (`iframe`) or
/// `object-src` (`object`, `embed`) loads *another* document, as does an `<a href>`, which
/// reaches no destination at all until it is activated; those stay out, because rewriting
/// them would change what the reference means rather than make the page self-contained.
///
/// Every name is lower-case and no qualifier is namespaced, both deliberately: a foreign
/// element such as SVG's `<image>` is matched case-sensitively by local name, and an
/// invalid qualifier would throw where this string is used, aborting collection for the
/// whole page instead of for one element.
const ASSET_SELECTOR: &str = "img,image,input[type=image i],video,audio,source,track,\
                              link[rel*=icon],link[imagesrcset]";
/// Attributes the recreation re-derives, so recording them would fight the generator.
const SKIPPED_ATTRIBUTES: [&str; 3] = ["style", "nonce", "integrity"];

/// Localises every URL an attribute value contains, leaving everything else byte-identical.
///
/// The value is read as candidates, in the two positions the HTML srcset grammar defines.
/// In URL position a URL runs to the next whitespace, so a comma inside it is ordinary
/// data and only trailing commas end the candidate. In descriptor position the next comma
/// ends it. A lone URL is the one-candidate case of the same rule.
///
/// Every byte that is not a matched URL is copied through, so a value holding no asset
/// URLs — `sizes`, or any attribute that never named one — comes back unchanged.
pub fn rewrite(value: &str, assets: &BTreeMap<String, String>) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;
    while !rest.is_empty() {
        let start = rest
            .find(|character: char| !character.is_whitespace() && character != ',')
            .unwrap_or(rest.len());
        let (separator, tail) = rest.split_at(start);
        output.push_str(separator);
        rest = tail;
        if rest.is_empty() {
            break;
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let (token, tail) = rest.split_at(end);
        let url = token.trim_end_matches(',');
        output.push_str(assets.get(url).map(String::as_str).unwrap_or(url));
        output.push_str(&token[url.len()..]);
        rest = tail;
        if url.len() < token.len() {
            continue;
        }
        let end = rest.find(',').unwrap_or(rest.len());
        let (descriptors, tail) = rest.split_at(end);
        output.push_str(descriptors);
        rest = tail;
    }
    output
}

/// The same rule as JavaScript, for the injected capture scripts. Defines
/// `recreateAttributes(element, path)` and `recreateAssetUrls(nodes, cssRules)`, over the
/// surface reader it delegates an element's unaddressed content to.
pub fn js_source() -> String {
    let quoted = |names: &[&str]| {
        names
            .iter()
            .map(|name| format!("'{name}'"))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "{}{}",
        crate::css_url_text::js_source().to_string() + &crate::surface_content::js_source(),
        JS_SOURCE
            .replace("__URL_ATTRIBUTES__", &quoted(&URL_ATTRIBUTES))
            .replace("__CANDIDATE_ATTRIBUTES__", &quoted(&CANDIDATE_ATTRIBUTES))
            .replace("__SKIPPED_ATTRIBUTES__", &quoted(&SKIPPED_ATTRIBUTES))
            .replace("__ASSET_SELECTOR__", ASSET_SELECTOR)
    )
}

const JS_SOURCE: &str = include_str!("asset_attributes.js");

#[cfg(test)]
#[path = "asset_attributes_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "asset_attributes_gate_tests.rs"]
mod gate_tests;

#[cfg(test)]
#[path = "asset_attributes_reach_harness.rs"]
mod reach_harness;

#[cfg(test)]
#[path = "asset_attributes_reach_tests.rs"]
mod reach_tests;

#[cfg(test)]
#[path = "asset_attributes_rule_text_tests.rs"]
mod rule_text_tests;

#[cfg(test)]
#[path = "asset_attributes_pseudo_tests.rs"]
mod pseudo_tests;

#[cfg(test)]
#[path = "asset_attributes_url_tests.rs"]
mod url_tests;

#[cfg(test)]
#[path = "asset_attributes_base_tests.rs"]
mod base_tests;

/// Which base a `url()` inside a stylesheet rule resolves against, which is the sheet's own
/// location rather than the document's.
#[cfg(test)]
#[path = "asset_attributes_sheet_base_tests.rs"]
mod sheet_base_tests;
