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
    selectors.split(',').any(|selector| {
        let selector = selector.trim();
        let compound = terminal_compound(selector);
        if compound != selector {
            return false;
        }
        let required = compound_classes(compound);
        let tag = compound
            .chars()
            .take_while(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '*')
            })
            .collect::<String>();
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

fn terminal_compound(selector: &str) -> &str {
    selector
        .trim()
        .rsplit(|character: char| character.is_whitespace() || matches!(character, '>' | '+' | '~'))
        .find(|part| !part.is_empty())
        .unwrap_or_default()
}

fn compound_classes(compound: &str) -> Vec<String> {
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

fn compound_id(compound: &str) -> Option<&str> {
    let remaining = compound.split_once('#')?.1;
    let length = remaining
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .map(char::len_utf8)
        .sum();
    (length > 0).then_some(&remaining[..length])
}

fn compound_attributes(compound: &str) -> Vec<(&str, Option<&str>)> {
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
