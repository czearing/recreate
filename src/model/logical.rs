use super::WritingMode;

/// What a property name resolves to once the writing mode and direction in force are
/// applied to it.
///
/// Resolution is not a rename, so the cases differ in how MANY declarations they stand for
/// rather than only in which name. An axis names both of its edges, and a variant carrying
/// one name could not say so: the only answer left to it would be "nothing", which every
/// caller reads as a declaration to drop.
#[derive(Debug, PartialEq, Eq)]
pub enum Physical {
    /// The name is not logical and stands for itself.
    Same,
    /// A logical name that this element's writing mode and direction resolve to one
    /// physical name carrying the same value.
    Named(String),
    /// A logical name covering both edges of an axis, resolved to the physical name of each
    /// edge in start-then-end order. `margin-inline` is `margin-left` and `margin-right`
    /// under a horizontal left-to-right mode, and the pair reverses with the mode.
    Axis(String, String),
}

impl Physical {
    /// The physical declarations that `name: value` stands for.
    ///
    /// An axis shorthand takes one component or two: two name the edges in start-then-end
    /// order, and one names both. Any other count belongs to a family whose shorthand
    /// applies its whole value to each edge rather than dividing it — `border-inline: 1px
    /// solid red` draws that border on both edges — so the value is passed on intact. That
    /// is a reading of the value's arity, not a list of the names it may be spelled with.
    pub fn into_declarations(self, name: &str, value: &str) -> Vec<(String, String)> {
        let (start, end) = match self {
            Self::Same => return vec![(name.to_string(), value.to_string())],
            Self::Named(physical) => return vec![(physical, value.to_string())],
            Self::Axis(start, end) => (start, end),
        };
        let components = super::css_value::components(value);
        let (first, last) = match components.as_slice() {
            [first, last] => (*first, *last),
            _ => (value, value),
        };
        vec![(start, first.to_string()), (end, last.to_string())]
    }

    /// Whether an authored declaration spelled `name` answers a query for `property`.
    ///
    /// This is a different question from [`Self::into_declarations`] and the two part
    /// company on the axis case. Emission asks what the declaration resolves to, and an
    /// axis resolves to both of its edges; a query names the declaration it is looking for,
    /// and a shorthand is still the declaration that is there under the name it was
    /// written with.
    pub fn answers(&self, name: &str, property: &str) -> bool {
        match self {
            Self::Named(physical) => physical == property,
            Self::Same | Self::Axis(..) => name == property,
        }
    }
}

/// Resolve a property name against the writing mode and direction in force at the element.
///
/// CSS Logical spells every box property the same way — a family, an axis, and an edge —
/// so this reads the grammar rather than listing the names. Listing them would cover the
/// families that exist today and silently drop whichever the specification adds next,
/// which is the failure that produced this function.
pub fn physical_property(mode: WritingMode, rtl: bool, name: &str) -> Physical {
    let size = mode.physical_size(name);
    if !size.is_empty() {
        return Physical::Named(size.to_string());
    }
    let parts: Vec<&str> = name.split('-').collect();
    corner_radius(mode, rtl, &parts).unwrap_or_else(|| edge_property(mode, rtl, &parts))
}

/// The physical edge one end of one axis lands on.
///
/// `direction` reaches the inline axis alone. It chooses which end of that axis is its
/// start and says nothing about block flow, so passing it to both would mirror pages
/// vertically the moment one of them declared `direction: rtl`.
fn edge(mode: WritingMode, rtl: bool, axis: &str, side: &str) -> Option<&'static str> {
    let (start, end) = match axis {
        "block" => mode.block_edges(),
        "inline" => mode.inline_edges(rtl),
        _ => return None,
    };
    match side {
        "start" => Some(start),
        "end" => Some(end),
        _ => None,
    }
}

/// `margin-inline-start`, `padding-block-end`, `border-inline-start-width`,
/// `scroll-padding-block-start` — one family, one axis, one edge, and any suffix the
/// family carries. Where no edge token follows the axis, the name covers both edges of it.
fn edge_property(mode: WritingMode, rtl: bool, parts: &[&str]) -> Physical {
    let Some(axis_at) = parts.iter().position(|p| matches!(*p, "block" | "inline")) else {
        return Physical::Same;
    };
    let axis = parts[axis_at];
    if let Some(side) = parts.get(axis_at + 1)
        && let Some(edge) = edge(mode, rtl, axis, side)
    {
        return Physical::Named(composed(parts, axis_at, edge, &parts[axis_at + 2..]));
    }
    // `margin-inline` and `border-block-width` name both edges of an axis. The token after
    // the axis is a suffix the family carries rather than a side, so it is kept on both.
    let suffix = &parts[axis_at + 1..];
    let (Some(start), Some(end)) = (edge(mode, rtl, axis, "start"), edge(mode, rtl, axis, "end"))
    else {
        return Physical::Same;
    };
    Physical::Axis(
        composed(parts, axis_at, start, suffix),
        composed(parts, axis_at, end, suffix),
    )
}

/// The physical name a family, an edge and a suffix spell together.
///
/// `inset-inline-start` names `left`, not `inset-left`: this family drops its prefix
/// instead of keeping it, which is the one place the grammar is irregular.
fn composed(parts: &[&str], axis_at: usize, edge: &str, suffix: &[&str]) -> String {
    let family = &parts[..axis_at];
    let head: &[&str] = if family == ["inset"] { &[] } else { family };
    head.iter()
        .copied()
        .chain([edge])
        .chain(suffix.iter().copied())
        .collect::<Vec<_>>()
        .join("-")
}

/// `border-start-start-radius` names a corner, so it composes one edge from each axis
/// rather than renaming one edge. The block side is written first and the inline side
/// second.
fn corner_radius(mode: WritingMode, rtl: bool, parts: &[&str]) -> Option<Physical> {
    let ["border", block, inline, "radius"] = parts else {
        return None;
    };
    let block = edge(mode, rtl, "block", block)?;
    let inline = edge(mode, rtl, "inline", inline)?;
    // `border-top-left-radius` names the vertical edge first and the horizontal one second.
    // Under a vertical writing mode the block axis IS the horizontal one, so the pair is
    // ordered by the edge each answer landed on, never by the axis that produced it.
    let (vertical, horizontal) = if mode.horizontal() {
        (block, inline)
    } else {
        (inline, block)
    };
    Some(Physical::Named(format!(
        "border-{vertical}-{horizontal}-radius"
    )))
}

#[cfg(test)]
#[path = "logical_tests.rs"]
mod logical_tests;

#[cfg(test)]
#[path = "logical_axis_tests.rs"]
mod logical_axis_tests;
