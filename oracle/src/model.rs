use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Environment {
    pub schema: u32,
    pub browser_product: String,
    pub browser_revision: String,
    pub protocol_version: String,
    pub command_line_digest: String,
    pub operating_system: String,
    pub architecture: String,
    pub locale: String,
    pub timezone: String,
    pub color_scheme: String,
    pub reduced_motion: bool,
    pub device_scale_factor_milli: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Scenario {
    pub id: String,
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Step {
    Reset,
    SetViewport { width: u32, height: u32 },
    ResizePath { widths: Vec<u32>, height: u32 },
    AdvanceTime { milliseconds: u32 },
    Activate { anchor: String },
    Hover { anchor: String },
    Key { key: String },
    SeekAnimations { milliseconds: u32 },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Checkpoint {
    pub scenario: String,
    pub step: usize,
    pub viewport: Viewport,
    pub domains: BTreeMap<String, Domain>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Domain {
    pub digest: String,
    pub value: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Artifact {
    pub format: String,
    pub source: String,
    pub environment: Environment,
    pub scenarios: Vec<Scenario>,
    pub obligations: Vec<Obligation>,
    pub checkpoints: Vec<Checkpoint>,
    pub coverage: Coverage,
    pub payload_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Obligation {
    pub id: String,
    pub kind: String,
    pub status: ObligationStatus,
    pub scenarios: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObligationStatus {
    Qualified,
    UnreachableProven,
    Opaque,
    Uncovered,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Coverage {
    pub widths_required: usize,
    pub widths_observed: usize,
    pub domains_required: Vec<String>,
    pub incomplete: Vec<String>,
}
