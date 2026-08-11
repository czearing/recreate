use super::selector_list;
use crate::model::Node;
pub(super) fn resolved_matches(node: &Node, name: &str, value: &str) -> bool {
    if matches!(name, "width" | "height") && value == "auto" {
        return node
            .style
            .get(name)
            .is_none_or(|computed| computed == value);
    }
    if !matches!(
        name,
        "align-content"
            | "align-items"
            | "align-self"
            | "column-gap"
            | "display"
            | "flex-direction"
            | "flex-flow"
            | "flex-wrap"
            | "gap"
            | "justify-content"
            | "justify-items"
            | "justify-self"
            | "order"
            | "position"
            | "row-gap"
            | "white-space"
    ) {
        return true;
    }
    node.style
        .get(name)
        .is_none_or(|computed| computed == value)
}

pub(super) fn directly_targets_node(selectors: &str, node: &Node) -> bool {
    let classes = node
        .attributes
        .get("class")
        .into_iter()
        .flat_map(|value| value.split_whitespace())
        .collect::<std::collections::HashSet<_>>();
    selector_list::members(selectors).any(|selector| {
        let compound = terminal_compound(selector);
        if compound != selector {
            return false;
        }
        let required = compound_classes(compound);
        let tag = compound_tag(compound);
        let id = compound_id(compound);
        let attributes = compound_attributes(compound);
        let constrained =
            !required.is_empty() || !tag.is_empty() || id.is_some() || !attributes.is_empty();
        constrained
            && (tag.is_empty() || tag == "*" || tag == node.tag)
            && id.is_none_or(|id| node.attributes.get("id").is_some_and(|value| value == id))
            && attributes.iter().all(|(name, expected)| {
                node.attributes
                    .get(*name)
                    .is_some_and(|actual| expected.is_none_or(|expected| actual == expected))
            })
            && required
                .iter()
                .all(|class| classes.contains(class.as_str()))
    })
}

/// The selector parser below is the single owner of what a selector means. Both the direct
/// matcher above and the authored-value index read compounds through these functions.
pub(super) fn terminal_compound(selector: &str) -> &str {
    selector
        .trim()
        .rsplit(|character: char| character.is_whitespace() || matches!(character, '>' | '+' | '~'))
        .find(|part| !part.is_empty())
        .unwrap_or_default()
}

pub(super) fn compound_tag(compound: &str) -> &str {
    let length = compound
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '*'))
        .map(char::len_utf8)
        .sum();
    &compound[..length]
}

pub(super) fn compound_classes(compound: &str) -> Vec<String> {
    let mut classes = Vec::new();
    let mut remaining = compound;
    while let Some(index) = remaining.find('.') {
        remaining = &remaining[index + 1..];
        let length = remaining
            .chars()
            .take_while(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            })
            .map(char::len_utf8)
            .sum();
        if length == 0 {
            break;
        }

        classes.push(remaining[..length].to_string());
        remaining = &remaining[length..];
    }

    classes
}

pub(super) fn compound_id(compound: &str) -> Option<&str> {
    let remaining = compound.split_once('#')?.1;
    let length = remaining
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .map(char::len_utf8)
        .sum();
    (length > 0).then_some(&remaining[..length])
}

pub(super) fn compound_attributes(compound: &str) -> Vec<(&str, Option<&str>)> {
    let mut attributes = Vec::new();
    let mut remaining = compound;
    while let Some((_, after_open)) = remaining.split_once('[') {
        let Some((attribute, after_close)) = after_open.split_once(']') else {
            break;
        };
        let (name, value) = attribute
            .split_once('=')
            .map_or((attribute, None), |(name, value)| {
                (
                    name,
                    Some(
                        value
                            .trim()
                            .trim_matches(|character| matches!(character, '"' | '\'')),
                    ),
                )
            });
        let name = name.trim();
        if !name.is_empty() {
            attributes.push((name, value));
        }
        remaining = after_close;
    }
    attributes
}

/// An authored value that references a custom property is normally dropped in favour of the
/// sampled computed value, which is safe for a colour or a spacing but destroys layout that
/// responds to the viewport: a track list like
/// `repeat(auto-fit, minmax(min(var(--min-col), 100%), 1fr))` samples as `230px 230px 230px
/// 230px`, freezing the column count at every width. Custom properties are re-emitted per
/// element with their resolved values, so such a value still resolves in the recreation and
/// is kept instead.
pub(super) fn fluid_authored_value(value: &str) -> bool {
    ["repeat(", "minmax(", "auto-fit", "auto-fill", "clamp(", "%"]
        .into_iter()
        .any(|token| value.contains(token))
}

/// Authored stylesheets commonly size boxes with logical properties, while the
/// generator reasons in physical ones. Without this mapping the authored value
/// is discarded and a sampled pixel width freezes the layout.
pub(super) fn physical_property(node: &Node, name: &str) -> &'static str {
    let horizontal = node
        .style
        .get("writing-mode")
        .is_none_or(|mode| mode.starts_with("horizontal"));
    if !horizontal {
        return "";
    }
    match name {
        "inline-size" => "width",
        "min-inline-size" => "min-width",
        "max-inline-size" => "max-width",
        "block-size" => "height",
        "min-block-size" => "min-height",
        "max-block-size" => "max-height",
        _ => "",
    }
}

/// A CSS-wide keyword is a cascade instruction, not a value. `all: unset` says
/// "resolve every property as if nothing declared it", so it names no geometry and
/// cannot be the authored source of a computed pixel.
///
/// Emitting one is worse than emitting nothing. The generator writes an element's
/// resolved longhands into a base rule and its per-viewport rules into `@media`
/// bands, and those bands come last. A band that emits `padding: unset` therefore
/// overrides the base rule's correct `padding-left: 10px`, resetting it to zero.
/// Dropping the keyword leaves the property undeclared in the band, so the base
/// rule keeps winning — which is the behaviour the keyword was asking for anyway.
pub(super) fn cascade_keyword(value: &str) -> bool {
    matches!(
        value.trim().trim_end_matches('}').trim(),
        "unset" | "initial" | "inherit" | "revert" | "revert-layer"
    )
}

/// The capture enumerates longhands only, so every shorthand in a style map came from
/// [`retained`] above. Declarations are emitted in sorted order, which puts a shorthand
/// ahead of the longhands that spell it out, so once those longhands are present the
/// shorthand is overridden on the very next line and carries no meaning — it only
/// repeats the value in a second syntax. This drops the inert copy. It is derived from
/// [`retained`] rather than stated separately so the two cannot drift apart: a name is a
/// shorthand exactly when other retained names, or the longhands listed here for the two
/// families whose parts are not spelled as their prefix, expand it.
pub(super) fn overridden_shorthand(name: &str, has: impl Fn(&str) -> bool) -> bool {
    let parts: &[&str] = match name {
        "gap" => &["row-gap", "column-gap"],
        "inset" => &["top", "right", "bottom", "left"],
        _ => &[],
    };
    if !parts.is_empty() {
        return parts.iter().all(|part| has(part));
    }
    retained(name)
        && SHORTHAND_PARTS.iter().any(|part| {
            part.strip_prefix(name)
                .is_some_and(|rest| rest.starts_with('-') && has(part))
        })
}

/// The longhand names a [`retained`] shorthand can be overridden by.
const SHORTHAND_PARTS: &[&str] = &[
    "flex-basis",
    "flex-direction",
    "flex-grow",
    "flex-shrink",
    "flex-wrap",
    "margin-bottom",
    "margin-left",
    "margin-right",
    "margin-top",
    "overflow-x",
    "overflow-y",
    "padding-bottom",
    "padding-left",
    "padding-right",
    "padding-top",
    "transition-behavior",
    "transition-delay",
    "transition-duration",
    "transition-property",
    "transition-timing-function",
];

pub(super) fn retained(name: &str) -> bool {
    matches!(
        name,
        "align-content"
            | "align-items"
            | "align-self"
            | "bottom"
            | "box-sizing"
            | "column-gap"
            | "display"
            | "flex"
            | "flex-basis"
            | "flex-direction"
            | "flex-flow"
            | "flex-grow"
            | "flex-shrink"
            | "flex-wrap"
            | "gap"
            | "grid-auto-columns"
            | "grid-auto-flow"
            | "grid-auto-rows"
            | "grid-column"
            | "grid-row"
            | "grid-template-columns"
            | "grid-template-rows"
            | "height"
            | "inset"
            | "justify-content"
            | "justify-items"
            | "justify-self"
            | "left"
            | "margin"
            | "margin-bottom"
            | "margin-left"
            | "margin-right"
            | "margin-top"
            | "max-height"
            | "max-width"
            | "min-height"
            | "min-width"
            | "object-fit"
            | "opacity"
            | "order"
            | "overflow"
            | "overflow-x"
            | "overflow-y"
            | "padding"
            | "padding-bottom"
            | "padding-left"
            | "padding-right"
            | "padding-top"
            | "perspective-origin"
            | "position"
            | "right"
            | "row-gap"
            | "top"
            | "transform"
            | "transform-origin"
            | "transition"
            | "translate"
            | "white-space"
            | "width"
            | "z-index"
    )
}
