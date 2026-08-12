use super::PageState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionAction {
    #[default]
    Activate,
    Hover,
    Leave,
    Focus,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InteractionTransition {
    pub from_state: usize,
    pub to_state: usize,
    #[serde(default)]
    pub action: InteractionAction,
    pub trigger_path: String,
    pub trigger_tag: String,
    pub trigger_label: String,
    #[serde(default)]
    pub trigger_occurrence: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Interaction {
    pub trigger_path: String,
    pub trigger_tag: String,
    pub trigger_label: String,
    #[serde(default)]
    pub trigger_occurrence: Option<usize>,
    #[serde(default)]
    pub focused_path: Option<String>,
    pub states: Vec<PageState>,
}
