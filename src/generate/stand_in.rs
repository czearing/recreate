//! The single owner of what a stand-in may assert about the element it replaced.
//!
//! Two elements are replaced by an `<img>`: an inline `<svg>` past the reuse threshold,
//! written out as an asset, and a drawing surface whose painted content was read at
//! capture time. Both are substitutions, not moves, so the question is not which
//! attributes to keep but which ones are still *true*. The replaced element is gone: an
//! attribute describing **it** is now a claim about nothing, while an attribute describing
//! the content it showed, or the box it occupied, is still true of the stand-in, which
//! shows the same graphic in the same place.
//!
//! Neither list shape states that, which is why this is not one. An allow-list drops the
//! names nobody recalled — it carried `aria-hidden`, the one accessibility attribute whose
//! meaning already matched the hardcoded `alt={""}` beside it, and dropped every attribute
//! that named the graphic. A denylist, the crate's usual shape in `jsx_attrs`, carries the
//! source vocabulary — `viewBox`, `fill`, `d`, `xmlns` — onto an HTML host that ignores it.
//!
//! Crossing is also not copying. Referenced through `<img src>` the graphic becomes an
//! image document: its internal `role`, `title` and `aria-*` are never exposed to assistive
//! technology, so the naming bytes survive in the asset and reach nobody. A name has to
//! change channel to survive the substitution, and `alt` is the only one the new element
//! still exposes.

use super::jsx_markup::root_attributes;
use crate::model::Node;
use std::collections::BTreeMap;

/// The attributes the replaced element and its stand-in both define. Their subject is the
/// box, which the substitution does not move.
const BOX_ATTRIBUTES: [&str; 3] = ["className", "height", "width"];

/// The stand-in for a graphic relocated to `filename`.
pub(super) fn image(svg: &str, filename: &str) -> String {
    let root = root_attributes(svg);
    let attributes = root
        .iter()
        .filter(|(found, _)| carried(found))
        .map(|(name, value)| format!(" {name}={}", literal(value)))
        .collect::<String>();
    format!(
        "<img src={} alt={}{attributes} />",
        literal(&format!("/assets/{filename}")),
        literal(&name(
            root.iter().map(|(name, value)| (name.as_str(), value))
        ))
    )
}

/// Where the element's painted content ended up, for an element that had some and whose
/// bytes reached the project. A surface whose content could not be read never carried the
/// key, and one whose bytes never arrived resolves to nothing here — in both cases the
/// element is emitted as itself, at its measured size, exactly as it was before any of
/// this existed. A stand-in pointing at a file that is not there would be worse than the
/// empty box it replaced.
pub(super) fn painted_source<'a>(
    node: &Node,
    assets: &'a BTreeMap<String, String>,
) -> Option<&'a String> {
    assets.get(node.attributes.get(crate::surface_content::ATTRIBUTE)?)
}

/// The element a node is emitted as. Keyed on whether the node carries painted content
/// rather than on its tag, so the emitter holds no notion of which elements are drawing
/// surfaces: the capture decided that by reading one, and an element that never painted
/// anything is emitted as itself.
pub(super) fn tag<'a>(node: &'a Node, assets: &BTreeMap<String, String>) -> &'a str {
    match painted_source(node, assets) {
        Some(_) => "img",
        None => super::jsx_attrs::jsx_tag(&node.tag),
    }
}

/// The key is never emitted. Once resolved it is the stand-in's source; unresolved it names
/// bytes the project does not have, and an attribute the browser cannot use.
pub(super) fn suppressed(name: &str, source: Option<&String>) -> bool {
    name == crate::surface_content::ATTRIBUTE || (source.is_some() && name == "aria-label")
}

/// What the stand-in adds to the attributes the replaced element already defined.
pub(super) fn rendered(node: &Node, source: Option<&String>) -> String {
    let Some(source) = source else {
        return String::new();
    };
    let attributes = node
        .attributes
        .iter()
        .map(|(name, value)| (name.as_str(), value));
    format!(
        " src={} alt={}",
        super::jsx_attrs::quoted(source),
        super::jsx_attrs::quoted(&name(attributes))
    )
}

/// `aria-*` describes the content, which the stand-in still shows, so it crosses as a class
/// rather than as a set of remembered names — an attribute this crate has never met is
/// carried with no second code change. `aria-label` is the exception, and is translated
/// rather than carried: `alt={""} aria-label={"…"}` is valid and the label would win, but
/// ARIA-in-HTML discourages overriding `alt` on an image and the pair reads as decorative
/// to every linter while announcing text. `aria-labelledby` outranks `alt` and its IDREFs
/// still resolve in the host document, so it crosses untouched.
///
/// `role` falls outside by construction rather than by exclusion: it names the semantics of
/// the element that no longer exists, an `<img>` already asserts the image role, and a
/// copied `presentation` would contradict a name on the same tag.
fn carried(name: &str) -> bool {
    (name.starts_with("aria-") && name != "aria-label") || BOX_ATTRIBUTES.contains(&name)
}

/// The one name the stand-in still exposes, taken from the one the replaced element
/// exposed in a channel an `<img>` no longer has.
fn name<'a>(attributes: impl Iterator<Item = (&'a str, &'a String)>) -> String {
    attributes
        .filter(|(name, _)| *name == "aria-label")
        .map(|(_, value)| value.clone())
        .next()
        .unwrap_or_default()
}

fn literal(value: &str) -> String {
    format!("{{{}}}", serde_json::to_string(value).unwrap())
}

#[cfg(test)]
#[path = "stand_in_tests.rs"]
mod tests;
