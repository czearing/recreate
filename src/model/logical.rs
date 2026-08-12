use super::WritingMode;

/// What a property name resolves to once the writing mode and direction in force are
/// applied to it.
///
/// The three cases exist because two of them used to share one empty-string answer, and a
/// caller cannot act correctly on the pair. "Already physical" is safe to pass on to an
/// allow-list of physical names; "logical, and this build has no single physical name for
/// it" is guaranteed to be rejected there, so passing it on drops the declaration and
/// substitutes whatever value was sampled. Spelling the second case out turns a silent
/// loss into a condition a test can name.
#[derive(Debug, PartialEq, Eq)]
pub enum Physical {
    /// The name is not logical and stands for itself.
    Same,
    /// A logical name that this element's writing mode and direction resolve to one
    /// physical name carrying the same value.
    Named(String),
    /// A logical name with no single physical equivalent. Every such name is a shorthand
    /// over both edges of an axis, so honouring it means splitting one declaration into
    /// two rather than renaming it.
    Unsupported,
}

impl Physical {
    /// The physical name to emit `name` under, or `None` when the declaration cannot be
    /// carried across as a single one.
    pub fn into_name(self, name: &str) -> Option<String> {
        match self {
            Self::Same => Some(name.to_string()),
            Self::Named(physical) => Some(physical),
            Self::Unsupported => None,
        }
    }

    /// Whether an authored declaration spelled `name` answers a query for `property`.
    ///
    /// This is a different question from [`Self::into_name`] and the two part company on the
    /// unsupported case. Emitting is a rename and a shorthand has no single name to be
    /// renamed to, so nothing is emitted; a query names the declaration it is looking for,
    /// and a shorthand is still the declaration that is there.
    pub fn answers(&self, name: &str, property: &str) -> bool {
        match self {
            Self::Named(physical) => physical == property,
            Self::Same | Self::Unsupported => name == property,
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
/// family carries.
fn edge_property(mode: WritingMode, rtl: bool, parts: &[&str]) -> Physical {
    let Some(axis_at) = parts.iter().position(|p| matches!(*p, "block" | "inline")) else {
        return Physical::Same;
    };
    // `margin-inline` and `border-block-width` name both edges of an axis and carry one or
    // two values. Renaming either would assign a pair to a single edge.
    let Some(side) = parts.get(axis_at + 1) else {
        return Physical::Unsupported;
    };
    let Some(edge) = edge(mode, rtl, parts[axis_at], side) else {
        return Physical::Unsupported;
    };
    // `inset-inline-start` names `left`, not `inset-left`: this family drops its prefix
    // instead of keeping it, which is the one place the grammar is irregular.
    let family = &parts[..axis_at];
    let head: &[&str] = if family == ["inset"] { &[] } else { family };
    let resolved: Vec<&str> = head
        .iter()
        .copied()
        .chain([edge])
        .chain(parts[axis_at + 2..].iter().copied())
        .collect();
    Physical::Named(resolved.join("-"))
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
