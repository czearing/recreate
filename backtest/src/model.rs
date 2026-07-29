use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Session {
    pub schema_version: u32,
    pub side: String,
    pub cdp_url: String,
    pub target_id: String,
    pub requested_url: String,
    pub rendered_url: String,
    pub browser: String,
    pub viewport: Viewport,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Artifact {
    pub schema_version: u32,
    pub source: SourceIdentity,
    pub states: Vec<StateEvidence>,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceIdentity {
    pub requested_url: String,
    pub rendered_url: String,
    pub browser: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StateEvidence {
    pub viewport: Viewport,
    pub baseline: Snapshot,
    pub scenarios: Vec<ScenarioEvidence>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScenarioEvidence {
    pub id: String,
    pub action: Action,
    pub checkpoints: Vec<Checkpoint>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Checkpoint {
    pub virtual_ms: u64,
    pub snapshot: Snapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Click,
    Hover,
    Focus,
    Input,
    Escape,
    Timer,
}

impl ActionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::Hover => "hover",
            Self::Focus => "focus",
            Self::Input => "input",
            Self::Escape => "escape",
            Self::Timer => "timer",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Action {
    pub kind: ActionKind,
    pub target: String,
    pub label: String,
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Snapshot {
    pub url: String,
    pub title: String,
    pub document: [f64; 2],
    pub nodes: Vec<NodeSnapshot>,
    pub animations: Vec<AnimationSnapshot>,
    pub active: Option<String>,
    pub pixel_hash: String,
    pub screenshot_png: String,
    pub console_errors: Vec<String>,
    pub network_failures: Vec<String>,
    pub unexpected_requests: Vec<String>,
    pub pending_requests: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NodeSnapshot {
    pub id: String,
    pub path: String,
    pub parent: Option<String>,
    pub tag: String,
    pub text: String,
    pub role: String,
    pub name: String,
    pub attributes: BTreeMap<String, String>,
    pub rect: [f64; 4],
    pub style: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AnimationSnapshot {
    pub target: String,
    pub duration: f64,
    pub delay: f64,
    pub iterations: String,
    pub direction: String,
    pub easing: String,
    pub fill: String,
    pub properties: Vec<String>,
    pub keyframes: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Pass,
    Fail,
    Inconclusive,
    PreparationRequired,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Report {
    pub status: Status,
    pub elapsed_ms: u128,
    pub findings: Vec<Finding>,
    pub coverage: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Finding {
    pub id: String,
    pub key: String,
    pub category: String,
    pub viewport: u32,
    pub checkpoint: String,
    pub action: String,
    pub target: String,
    pub property: String,
    pub source: String,
    pub candidate: String,
    pub delta: Option<String>,
    pub effects: Vec<String>,
}
