//! What the page was observed doing over time, as opposed to what it looked like at rest.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// How the element holding a state sits relative to the element the rule styles.
///
/// Selectors defines exactly one relational pseudo-class, so a state reaches its subject in
/// one of two ways and no more: from an ancestor, which a descendant combinator expresses,
/// or from inside the subject, which only `:has()` expresses. Recording which one is what
/// stops `.card:has(.button:hover)` being re-emitted as `.card:hover`, where the style
/// spreads over the whole card instead of answering the button.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    #[default]
    Ancestor,
    Contained,
}

impl Relation {
    pub fn is_ancestor(&self) -> bool {
        *self == Self::Ancestor
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct StateStyle {
    pub target: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "Relation::is_ancestor")]
    pub relation: Relation,
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
