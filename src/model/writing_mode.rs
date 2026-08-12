use serde::{Deserialize, Serialize};

/// The writing mode in force at an element when the page was read.
///
/// `writing-mode` is inherited, so a page declares it once on a wrapper and every box
/// inside it carries no declaration of its own. The authored style map records what the
/// author wrote and is right to leave those boxes empty, but a rule mapping a logical
/// dimension onto a physical one needs the value in effect at the box, which no record of
/// authored declarations can hold. Re-deriving it would mean walking ancestors from a rule
/// that is handed one node, so the answer is taken from the engine and recorded once,
/// exactly as `disabled` and `rtl` are. It is a fact about the element rather than a
/// declaration, and is never emitted as CSS.
///
/// The resolved keyword is kept rather than a `horizontal` flag because the vertical
/// values agree on the axis and disagree on the edge: `vertical-rl` and `vertical-lr` both
/// make the inline axis vertical, while placing block-start on opposite sides of the box.
/// A flag would carry the first distinction and lose the second, reproducing this same
/// collapse one level down for any rule that resolves a logical edge.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", from = "String")]
pub enum WritingMode {
    #[default]
    HorizontalTb,
    VerticalRl,
    VerticalLr,
    SidewaysRl,
    SidewaysLr,
}

impl From<String> for WritingMode {
    /// A keyword this build does not know resolves to the initial value, which is how an
    /// engine that does not implement it lays the page out.
    fn from(value: String) -> Self {
        match value.as_str() {
            "vertical-rl" => Self::VerticalRl,
            "vertical-lr" => Self::VerticalLr,
            "sideways-rl" => Self::SidewaysRl,
            "sideways-lr" => Self::SidewaysLr,
            _ => Self::HorizontalTb,
        }
    }
}

impl WritingMode {
    /// Whether the inline axis is the horizontal one. False under every vertical mode,
    /// including the `sideways-*` pair, which rotate glyphs but not the axes.
    ///
    /// This is also the initial value, so it doubles as the predicate that keeps the fact
    /// out of a spec for a page that never declared a writing mode.
    pub fn horizontal(&self) -> bool {
        *self == Self::HorizontalTb
    }

    /// The physical edges the block axis starts and ends on.
    ///
    /// `direction` is not an input. It chooses which end of the INLINE axis is its start
    /// and says nothing about block flow, which the writing mode fixes on its own.
    pub fn block_edges(&self) -> (&'static str, &'static str) {
        match self {
            Self::HorizontalTb => ("top", "bottom"),
            Self::VerticalRl | Self::SidewaysRl => ("right", "left"),
            Self::VerticalLr | Self::SidewaysLr => ("left", "right"),
        }
    }

    /// The physical edges the inline axis starts and ends on, given the direction in force.
    ///
    /// `sideways-lr` is the reason this cannot be derived from [`Self::horizontal`] or from
    /// the sizing table. Its inline flow runs bottom to top, while `vertical-lr` runs top to
    /// bottom, so the two agree on the axis and disagree on which end starts it. The sizing
    /// table is right to group them — both make the inline axis vertical, so `inline-size`
    /// is the height either way — and reusing that grouping for edges would mirror every
    /// box on a `sideways-lr` page.
    pub fn inline_edges(&self, rtl: bool) -> (&'static str, &'static str) {
        let (start, end) = match self {
            Self::HorizontalTb => ("left", "right"),
            Self::VerticalRl | Self::VerticalLr | Self::SidewaysRl => ("top", "bottom"),
            Self::SidewaysLr => ("bottom", "top"),
        };
        if rtl { (end, start) } else { (start, end) }
    }

    /// The physical dimension a logical sizing property names at this element, or an empty
    /// string for a property that is not one.
    ///
    /// Only the writing mode decides this. `direction` chooses which END of the inline axis
    /// is its start, never which axis it is, so it is not an input here. Under a vertical
    /// mode the two axes exchange places, which is why the pair transposes rather than
    /// degrading: `inline-size` becomes the height and `block-size` the width.
    pub fn physical_size(&self, name: &str) -> &'static str {
        const HORIZONTAL: [&str; 3] = ["width", "min-width", "max-width"];
        const VERTICAL: [&str; 3] = ["height", "min-height", "max-height"];
        let (inline, block) = if self.horizontal() {
            (HORIZONTAL, VERTICAL)
        } else {
            (VERTICAL, HORIZONTAL)
        };
        match name {
            "inline-size" => inline[0],
            "min-inline-size" => inline[1],
            "max-inline-size" => inline[2],
            "block-size" => block[0],
            "min-block-size" => block[1],
            "max-block-size" => block[2],
            _ => "",
        }
    }
}

#[cfg(test)]
#[path = "writing_mode_tests.rs"]
mod tests;
