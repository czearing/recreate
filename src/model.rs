use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

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
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Pseudo {
    pub content: String,
    pub style: Styles,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Animation {
    pub target: String,
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
pub struct PageState {
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub viewport: Viewport,
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub startup_nodes: Vec<Node>,
    #[serde(default)]
    pub startup_delay_ms: u64,
    #[serde(default)]
    pub startup_duration_ms: u64,
    pub animations: Vec<Animation>,
    #[serde(default)]
    pub state_styles: Vec<StateStyle>,
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Interaction {
    pub trigger_path: String,
    pub trigger_tag: String,
    pub trigger_label: String,
    #[serde(default)]
    pub focused_path: Option<String>,
    pub states: Vec<PageState>,
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
