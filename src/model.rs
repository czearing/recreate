use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod css_value;
mod interaction;
mod logical;
mod writing_mode;
pub use css_value::components as value_components;
pub use interaction::{Interaction, InteractionAction, InteractionTransition};
pub use logical::{Physical, physical_property};
pub use writing_mode::WritingMode;

pub type Styles = BTreeMap<String, String>;
pub type Attributes = BTreeMap<String, String>;

/// What an element's live state says that its markup default does not, keyed by the content
/// attribute it overrides. `None` records a default that was turned off, which an absent
/// entry cannot express: a checkbox authored `checked` and then cleared differs from one
/// that was never touched.
pub type ControlState = BTreeMap<String, Option<String>>;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    #[serde(default = "default_dpr")]
    pub dpr: f64,
}

fn default_dpr() -> f64 {
    1.0
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Node {
    pub path: String,
    pub parent: Option<String>,
    pub tag: String,
    pub text: String,
    pub attributes: Attributes,
    /// The live state of the IDL attributes that do not reflect a content attribute.
    ///
    /// For a form control the content attribute is the *default*, not the state: `value` is
    /// `defaultValue`, `checked` is `defaultChecked`, `selected` is `defaultSelected`. The
    /// user typing, or a script assigning the property, updates the state and never writes
    /// the attribute, so [`Node::attributes`] is structurally incapable of carrying it and
    /// no amount of settling puts it there. This is the same concession [`Node::disabled`]
    /// makes, applied to the rest of the family rather than to one member.
    ///
    /// Kept apart from the attribute map rather than folded into it because the default and
    /// the current state are different facts. Overwriting the map would make the record
    /// claim the page's markup said something it never said, and would break every consumer
    /// that legitimately wants the authored default.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub control_state: ControlState,
    pub rect: Rect,
    /// The space a classic scrollbar took out of this element's content box, in CSS pixels.
    ///
    /// Measured in the browser because it cannot be recovered afterwards. The engine reports
    /// a scrollbar by shrinking the resolved `width` of a content-box element, so restoring
    /// the authored size means adding the scrollbar back — but deriving it from the recorded
    /// geometry would need this element's padding and border, and `style` keeps only the
    /// declarations that differ from the element's baseline. A `<ul>` whose padding is the
    /// user-agent's own records no padding at all, and the missing term reads as a scrollbar
    /// that was never there. `offsetWidth - clientWidth` minus the two border widths is exact
    /// at capture time, where the computed style is still whole, and unlike
    /// `getBoundingClientRect` it is not scaled by an ancestor `transform`.
    #[serde(default, skip_serializing_if = "crate::model::is_zero")]
    pub scrollbar_gutter: f64,
    pub style: Styles,
    /// The properties an authored condition the recreation re-emits decided at this element,
    /// keyed by the chain of conditions that decided them, spelled as the text that opens it.
    ///
    /// [`Node::style`] is one branch of every condition the page carries — whichever branch
    /// held while the capture read it. The emitter republishes the condition, so it must take
    /// that branch back out of the unconditional rule, and the only sound proof that a
    /// condition put it there is the engine's: the capture withdraws the blocks of exactly
    /// the rules the emitter republishes, reads again, and records what moved.
    ///
    /// Recorded rather than re-derived because the two vocabularies do not meet. An authored
    /// `0.5em`, `5%`, `calc()`, `10cqw` or `teal` is never the string its own computed sample
    /// serialises to, so comparing them answers "was this spelled the way the engine spells
    /// it", not "did this declaration produce this value" — and no table of unit families
    /// closes that gap, because a percentage needs a containing block and a container unit
    /// needs the query container's used size.
    ///
    /// Keyed by the chain rather than pooled, because a property has to be put back under one
    /// particular prelude: an element sitting under two conditions would otherwise have every
    /// override restated under both, and the branch below one of them would paint the other's.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub condition_decided: BTreeMap<String, BTreeSet<String>>,
    /// What each decided property computes to once every republished condition is withdrawn —
    /// the arm the unconditional rule owes, measured rather than read off the authored text.
    ///
    /// The authored text answers first where it can, because the author's own words keep the
    /// output shaped like its source and say nothing where the author said nothing. It cannot
    /// answer where the value it states is itself a reference the recreation resolves
    /// elsewhere, or a share of a shorthand this stage cannot divide; there the engine's
    /// measurement is the only arm anybody has.
    #[serde(default, skip_serializing_if = "Styles::is_empty")]
    pub condition_base: Styles,
    /// The generated boxes this element had, keyed by the selector suffix that names each
    /// one — `::before`, `::after`, `::backdrop`.
    ///
    /// A map rather than one field per box. Every consumer hashes, diffs, rebases and emits
    /// these identically and none distinguishes them by meaning, so the count was never a
    /// fact about the record — only about which boxes were needed first. Spelling them as
    /// parallel fields made a further box an edit at every reader, where a reader that was
    /// missed keeps compiling and simply stops noticing that box changed.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pseudos: Pseudos,
    /// Whether the element was in the top layer when the page was read.
    ///
    /// The top layer is a user-agent list, not a property of any element: `showModal()`,
    /// `requestFullscreen()` and an invoked modal popover put an element in it, and nothing
    /// in the document declares that. The `open` content attribute cannot answer it, because
    /// `show()` and `showModal()` set that same attribute — it records that a dialog is being
    /// shown, not that it is modal. Computed style cannot either, and is worse than silent:
    /// the user-agent sheet gives `dialog:modal` its own `position` and `inset`, so a
    /// snapshot picks up plausible centring geometry and the record looks complete.
    ///
    /// This is the concession [`Node::disabled`] already makes, under a stronger premise.
    /// There, re-deriving the answer was merely awkward; here it is impossible, because no
    /// element anywhere in the document carries a value implying the promotion. `:modal` is
    /// specified to select exactly the elements excluding interaction with everything outside
    /// them, so the answer is taken from the engine and recorded once.
    ///
    /// It decides two things at once, which is why it is one field and not two: what the
    /// recreation must replay, and whether a `::backdrop` box exists to be recorded at all.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub modal: bool,
    /// Whether the element matched `:disabled` when the page was read.
    ///
    /// Disabled is not always borne by the element that shows it: a `<fieldset disabled>`
    /// disables every descendant control except those in its first `<legend>`, and the
    /// descendant carries no attribute of its own. Neither the attribute map nor the
    /// `disabled` DOM property — which reflects the content attribute and nothing else —
    /// can answer that, and re-deriving it here would mean re-implementing the rule and
    /// its carve-out. `:disabled` is specified to select exactly the set that is really
    /// disabled, so the answer is taken from the engine and recorded once.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
    /// Whether the element was a full-page blocking overlay when the page was read.
    ///
    /// The verdict is taken from the engine for the same reason `disabled` is. Whether an
    /// element hides the page behind it depends on `visibility`, which inherits, on
    /// `opacity`, which composites a subtree without inheriting, and on `content-visibility`,
    /// which is not a declaration in force at the element at all. A record of authored
    /// declarations holds none of those for a descendant that declared nothing, and reading
    /// their absence as evidence is what let a parked dialog be reported as a curtain. The
    /// rule is spelled once in [`crate::blocking_overlay`] and answered while the page is
    /// open. It is a fact about the element rather than a declaration, and is never emitted
    /// as CSS.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub blocking_overlay: bool,
    /// Whether the element's inline axis ran right-to-left when the page was read.
    ///
    /// `direction` is inherited, so a page declares it once at the root and every box it
    /// positions carries no declaration of its own. The authored style map records what
    /// the author wrote and is right to leave those boxes empty, but a rule that maps a
    /// logical edge onto a physical one needs the value in effect at the box, which no
    /// record of authored declarations can hold. Re-deriving it would mean walking
    /// ancestors from a rule that is handed one node, so the answer is taken from the
    /// engine and recorded once, exactly as `disabled` is. It is a fact about the
    /// element rather than a declaration, and is never emitted as CSS.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub rtl: bool,
    /// The writing mode in force at the element when the page was read.
    ///
    /// Recorded for the same reason as `rtl` above and read from the same computed style:
    /// `writing-mode` is inherited, so a page declares it once on a wrapper and the box
    /// whose logical size has to be resolved carries no declaration of its own. See
    /// [`WritingMode`] for why the resolved keyword is kept rather than an axis flag.
    #[serde(default, skip_serializing_if = "WritingMode::horizontal")]
    pub writing_mode: WritingMode,
}

fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct DomNode {
    pub namespace: String,
    pub node_type: u16,
    pub tree_scope: String,
    pub physical_parent: Option<String>,
    pub assigned_slot: Option<String>,
    pub shadow_root_mode: Option<String>,
    pub client_rects: Vec<Rect>,
    pub scroll_left: f64,
    pub scroll_top: f64,
    pub scroll_width: f64,
    pub scroll_height: f64,
    pub client_width: f64,
    pub client_height: f64,
    pub computed_style_properties: Vec<String>,
    pub computed_style_dictionary: Vec<String>,
    pub computed_style_values: Vec<u32>,
    pub custom_properties: Styles,
}

#[path = "model/behaviour.rs"]
mod behaviour;
#[path = "model/capture_result.rs"]
mod capture_result;
#[path = "model/pseudo.rs"]
mod pseudo;

/// Re-exported for the tests that build one directly; the record itself reaches every
/// consumer inside an [AttributeSequence].
#[allow(unused_imports)]
pub use behaviour::SequenceStep;
pub use behaviour::{Animation, AttributeSequence, StateStyle};
pub use capture_result::{Acceptance, BrowserCookie, PageState, Specification};
pub use pseudo::{Pseudo, Pseudos};
