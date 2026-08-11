//! The single owner of what a stand-in may assert about the element it replaced.
//!
//! An inline `<svg>` past the reuse threshold is written out as an asset and replaced by an
//! `<img>`. That is a substitution, not a move, so the question is not which attributes to
//! keep but which ones are still *true*. The replaced element is gone: an attribute
//! describing **it** is now a claim about nothing, while an attribute describing the
//! content it showed, or the box it occupied, is still true of the stand-in, which shows
//! the same graphic in the same place.
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

/// The attributes the replaced element and its stand-in both define. Their subject is the
/// box, which the substitution does not move.
const BOX_ATTRIBUTES: [&str; 3] = ["className", "height", "width"];

/// The stand-in for a graphic relocated to `filename`.
pub(super) fn image(svg: &str, filename: &str) -> String {
    let root = root_attributes(svg);
    let name = root
        .iter()
        .find(|(found, _)| found == "aria-label")
        .map(|(_, value)| value.as_str())
        .unwrap_or_default();
    let attributes = root
        .iter()
        .filter(|(found, _)| carried(found))
        .map(|(name, value)| format!(" {name}={}", literal(value)))
        .collect::<String>();
    format!(
        "<img src={} alt={}{attributes} />",
        literal(&format!("/assets/{filename}")),
        literal(name)
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

fn literal(value: &str) -> String {
    format!("{{{}}}", serde_json::to_string(value).unwrap())
}
