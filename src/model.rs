use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Node {
    pub path: String,
    pub parent: Option<String>,
    pub tag: String,
    pub text: String,
    pub attributes: Attributes,
    pub rect: Rect,
    pub style: Styles,
    pub before: Option<Pseudo>,
    pub after: Option<Pseudo>,
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Pseudo {
    pub content: String,
    pub style: Styles,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Animation {
    pub target: String,
    /// The `@keyframes` block this animation was declared with, empty for a script-driven
    /// animation the author never named.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub keyframes: Vec<Value>,
    pub timing: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct StateStyle {
    pub target: String,
    #[serde(default)]
    pub scope: Option<String>,
    pub pseudo: Option<String>,
    #[serde(default)]
    pub target_pseudo: Option<String>,
    pub media: Option<String>,
    pub declarations: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SequenceStep {
    pub value: String,
    pub delay_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AttributeSequence {
    pub target: String,
    pub attribute: String,
    pub values: Vec<String>,
    pub interval_ms: u64,
    #[serde(default)]
    pub steps: Vec<SequenceStep>,
    /// Whether the observed values came back round, which is the only fact separating a
    /// progression that should loop from one that should come to rest. `None` is a capture
    /// taken before this was recorded and says nothing either way, so it keeps looping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeats: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PageState {
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub viewport: Viewport,
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub dom: BTreeMap<String, DomNode>,
    #[serde(default)]
    pub capture_blockers: Vec<String>,
    #[serde(default)]
    pub startup_nodes: Vec<Node>,
    #[serde(default)]
    pub startup_delay_ms: u64,
    #[serde(default)]
    pub startup_duration_ms: u64,
    pub animations: Vec<Animation>,
    #[serde(default)]
    pub state_styles: Vec<StateStyle>,
    #[serde(default)]
    pub attribute_sequences: Vec<AttributeSequence>,
    pub css_rules: Vec<String>,
    pub asset_urls: Vec<String>,
    #[serde(default)]
    pub asset_data: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Specification {
    pub schema_version: u32,
    pub requested_url: String,
    pub captured_url: String,
    pub states: Vec<PageState>,
    #[serde(default)]
    pub interactions: Vec<Interaction>,
    #[serde(default)]
    pub transitions: Vec<InteractionTransition>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BrowserCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Acceptance {
    pub passed: bool,
    pub checks: BTreeMap<String, bool>,
    pub counts: BTreeMap<String, usize>,
}
