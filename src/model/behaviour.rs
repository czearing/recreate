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
/// A state reaches its subject either through a combinator or through a relational
/// pseudo-class, and Selectors defines four of the former and one of the latter. Recording
/// which one is what stops `.card:has(.button:hover)` being re-emitted as `.card:hover`,
/// where the style spreads over the whole card instead of answering the button — and stops
/// `.input:focus-visible ~ .indicator` being re-emitted as `.indicator:focus-visible`, which
/// puts a keyboard-focus ring on a decorative span that can never be focused.
///
/// The relation is read from the combinator the author wrote, never from the way the located
/// pair happens to sit. Widening it leaks a rule onto elements the author excluded; narrowing
/// it rewrites a descendant rule as a child rule whenever the holder is the direct parent.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    #[default]
    Ancestor,
    Parent,
    PreviousSibling,
    PrecedingSibling,
    Contained,
}

impl Relation {
    pub fn is_ancestor(&self) -> bool {
        *self == Self::Ancestor
    }

    /// The name the capture writes and the class minter folds in, spelled once.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ancestor => "ancestor",
            Self::Parent => "parent",
            Self::PreviousSibling => "previous_sibling",
            Self::PrecedingSibling => "preceding_sibling",
            Self::Contained => "contained",
        }
    }

    /// The rule that puts the state back where the author had it. Every combinator joins the
    /// holder to the subject in the order they were written; only containment reverses them,
    /// because CSS has no ancestor combinator and `:has()` is what it offers instead.
    pub fn join(self, holder: &str, target: &str) -> String {
        match self {
            Self::Ancestor => format!("{holder} {target}"),
            Self::Parent => format!("{holder}>{target}"),
            Self::PreviousSibling => format!("{holder}+{target}"),
            Self::PrecedingSibling => format!("{holder}~{target}"),
            Self::Contained => format!("{target}:has({holder})"),
        }
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
