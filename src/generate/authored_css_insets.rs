use super::Index;
use crate::model::{Node, Styles};

/// The two inset pairs, inline axis first.
const INSET_AXES: [(&str, &str); 2] = [("left", "right"), ("top", "bottom")];

/// An absolutely positioned box whose `left`, `width` and `right` are all non-auto is
/// over-constrained, and CSS 2.1 10.3.7 resolves that by re-solving `right` as `auto` in
/// LTR — discarding the AUTHOR'S declaration and keeping the tool's. Capture reads
/// computed style, which returns used pixels, so the generator cannot tell `right: 30%`
/// from an engine-derived length and emits the sampled `left` beside it. That pixel does
/// not duplicate the author's anchor, it deletes it, and the box becomes a step function
/// pinned to the capture viewports.
///
/// So drop a derived inset only on an axis the author anchored from the other side. The
/// removal is itself the repair: an absent inset is `auto`, which is the unknown the
/// over-constraint equation solves for, so the box re-derives its offset from the
/// author's anchor at every width instead of at one. Any literal — including a `calc()` —
/// would re-pin it to the capture.
///
/// Where the author anchored neither edge there is no competing declaration to destroy,
/// the box is not over-constrained, and the sampled pixel is the only positional
/// information in existence, so it is kept. Dropping insets unconditionally would break
/// every page the author positioned by sampling alone. Where the author anchored both,
/// the cascade is entirely theirs and is reproduced untouched.
pub(super) fn suppress_derived_insets(styles: &mut Styles, node: &Node, rules: &Index<'_>) {
    // The media-band caller passes a diff of the base styles, and `position` rarely
    // changes between viewports, so it is usually absent there. The captured computed
    // value is the authority on the positioning scheme either way.
    let position = styles
        .get("position")
        .or_else(|| node.style.get("position"))
        .map(String::as_str);
    if !matches!(position, Some("absolute" | "fixed")) {
        return;
    }
    // The physical edge a logical inset names depends on both `writing-mode` and
    // `direction`. Under a vertical writing mode the inline axis is the vertical one, so
    // a mapping written for horizontal text would arbitrate the wrong axis and could
    // delete a real anchor. Declining to act there leaves such pages exactly as before.
    if !node
        .style
        .get("writing-mode")
        .is_none_or(|mode| mode.starts_with("horizontal"))
    {
        return;
    }
    let rtl = node.rtl;
    // A shorthand re-states every edge it covers, so removing a longhand while leaving
    // `inset: 0 432px 48px 648px` beside it removes nothing. Split it into the edges it
    // stands for first; the two forms carry the same values, so this is a rewrite of one
    // declaration into four and cannot change the rendered box on its own.
    expand_inset_shorthands(styles, rtl);
    for (start, end) in INSET_AXES {
        let anchored = (
            authored_inset(node, rules, start, rtl),
            authored_inset(node, rules, end, rtl),
        );
        let derived = match anchored {
            (true, false) => end,
            (false, true) => start,
            _ => continue,
        };
        if styles
            .get(derived)
            .is_some_and(|value| value.ends_with("px"))
        {
            styles.remove(derived);
        }
    }
}

/// Rewrite `inset`, `inset-block` and `inset-inline` into the physical longhands they
/// stand for. A longhand already in the map came from the same sample and is at least as
/// specific, so it is left alone.
fn expand_inset_shorthands(styles: &mut Styles, rtl: bool) {
    let (inline_start, inline_end) = if rtl {
        ("right", "left")
    } else {
        ("left", "right")
    };
    for shorthand in ["inset", "inset-block", "inset-inline"] {
        let Some(value) = styles.get(shorthand).cloned() else {
            continue;
        };
        // A component may itself contain spaces inside `calc(...)`, which a plain
        // whitespace split would tear apart into values that mean something else.
        if value.contains('(') {
            continue;
        }
        let parts: Vec<&str> = value.split_whitespace().collect();
        let edges: Vec<(&str, &str)> = match (shorthand, parts.as_slice()) {
            ("inset", [all]) => ["top", "right", "bottom", "left"]
                .into_iter()
                .map(|edge| (edge, *all))
                .collect(),
            ("inset", [block, inline]) => vec![
                ("top", *block),
                ("bottom", *block),
                ("left", *inline),
                ("right", *inline),
            ],
            ("inset", [top, inline, bottom]) => vec![
                ("top", *top),
                ("bottom", *bottom),
                ("left", *inline),
                ("right", *inline),
            ],
            ("inset", [top, right, bottom, left]) => vec![
                ("top", *top),
                ("right", *right),
                ("bottom", *bottom),
                ("left", *left),
            ],
            ("inset-block", [all]) => vec![("top", *all), ("bottom", *all)],
            ("inset-block", [start, end]) => vec![("top", *start), ("bottom", *end)],
            ("inset-inline", [all]) => vec![(inline_start, *all), (inline_end, *all)],
            ("inset-inline", [start, end]) => {
                vec![(inline_start, *start), (inline_end, *end)]
            }
            _ => continue,
        };
        for (edge, edge_value) in edges {
            if !styles.contains_key(edge) {
                styles.insert(edge.into(), edge_value.into());
            }
        }
        styles.remove(shorthand);
    }
}

/// Whether the author anchored one physical edge, under any name that reaches it.
/// A lookup for the physical longhand alone reports `inset-inline-end: 30%` as
/// unauthored, which is the same as having no anchor and leaves the frozen pixel in
/// place — so the logical longhands and the `inset` shorthands are anchors too.
fn authored_inset(node: &Node, rules: &Index<'_>, edge: &str, rtl: bool) -> bool {
    let logical = match (edge, rtl) {
        ("left", false) | ("right", true) => ["inset-inline-start", "inset-inline"],
        ("left", true) | ("right", false) => ["inset-inline-end", "inset-inline"],
        ("top", _) => ["inset-block-start", "inset-block"],
        _ => ["inset-block-end", "inset-block"],
    };
    [edge, logical[0], logical[1], "inset"]
        .into_iter()
        .any(|name| rules.has_property(node, name))
}

#[cfg(test)]
#[path = "authored_css_insets_tests.rs"]
mod tests;
