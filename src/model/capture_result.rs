//! What one capture produced, and what a whole capture run produced.
//!
//! Kept apart from the element-level types so the two grow independently: the types above
//! describe a single box, and these describe the run that collected them.

use super::{
    Animation, AttributeSequence, DomNode, Interaction, InteractionTransition, Node, StateStyle,
    Viewport,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    /// How the engine divided each authored declaration block that sets a longhand it does
    /// not name, keyed by the block text as `css_rules` spells it. A block absent from here
    /// declared no shorthand; an empty share is a division the engine could not settle.
    #[serde(default)]
    pub css_shorthands: BTreeMap<String, BTreeMap<String, String>>,
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
