use crate::digest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Side {
    Source,
    Candidate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub schema_version: u32,
    pub side: Side,
    pub cdp_url: String,
    pub target_id: String,
    pub requested_url: String,
    pub rendered_url: String,
    pub browser: String,
    pub executable: String,
    pub profile: String,
    pub viewport: Viewport,
    pub digest: String,
}

impl Session {
    pub fn seal(&mut self) -> anyhow::Result<()> {
        self.digest.clear();
        self.digest = digest::json(self)?;
        Ok(())
    }

    pub fn verify(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.schema_version == SCHEMA_VERSION,
            "unsupported session schema"
        );
        anyhow::ensure!(!self.requested_url.is_empty(), "session URL is empty");
        anyhow::ensure!(!self.target_id.is_empty(), "session target is empty");
        let mut unsigned = self.clone();
        let expected = std::mem::take(&mut unsigned.digest);
        anyhow::ensure!(!expected.is_empty(), "session digest is missing");
        anyhow::ensure!(
            digest::json(&unsigned)? == expected,
            "session integrity check failed"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Action {
    Click { target: String },
    Hover { target: String },
    ClickSequence { targets: Vec<String>, label: String },
    Timer { milliseconds: u64, target: String },
    Animation { target: String },
}

impl Action {
    pub fn scenario(&self) -> String {
        match self {
            Self::Click { target } => format!("click:{target}"),
            Self::Hover { target } => format!("hover:{target}"),
            Self::ClickSequence { label, .. } => format!("click:{label}"),
            Self::Timer { milliseconds, .. } => format!("timer:{milliseconds}"),
            Self::Animation { target } => format!("animation:{target}"),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeEvidence {
    pub tag: String,
    pub parent: String,
    pub order: usize,
    pub text: String,
    pub visible: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub background: String,
    pub color: String,
    #[serde(default)]
    pub font_size: String,
    #[serde(default)]
    pub font_family: String,
    pub font_weight: String,
    #[serde(default)]
    pub line_height: String,
    pub border_color: String,
    pub border_radius: String,
    pub box_shadow: String,
    pub opacity: String,
    pub transform: String,
    pub role: String,
    pub accessible_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub raster_kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rendered_content_sha256: String,
    pub animated: bool,
    pub animation_duration_ms: Option<u64>,
    pub animation_delay_ms: Option<i64>,
    pub animation_easing: String,
    pub animation_direction: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub motions: Vec<MotionEvidence>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionEvidence {
    pub kind: String,
    pub name: String,
    pub duration_ms: u64,
    pub delay_ms: i64,
    pub end_delay_ms: i64,
    pub iterations: String,
    pub direction: String,
    pub fill: String,
    pub easing: String,
    pub properties: Vec<String>,
    pub checkpoints: Vec<MotionCheckpoint>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionCheckpoint {
    pub progress: u8,
    pub values: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEvidence {
    pub console_errors: Vec<String>,
    pub requests: Vec<String>,
    pub pending_timers: usize,
    pub pending_frames: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layout_shifts: Vec<LayoutShiftEvidence>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutShiftEvidence {
    pub value: f64,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RasterTileEvidence {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub viewport: Viewport,
    pub scenario: String,
    pub nodes: BTreeMap<String, NodeEvidence>,
    pub active_element: String,
    pub runtime: RuntimeEvidence,
    pub screenshot_sha256: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raster_tiles: Vec<RasterTileEvidence>,
    pub capture_complete: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceIdentity {
    pub requested_url: String,
    pub rendered_url: String,
    pub browser: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub schema_version: u32,
    pub source: SourceIdentity,
    pub actions: Vec<Action>,
    pub states: Vec<State>,
    pub digest: String,
}

impl Artifact {
    pub fn seal(&mut self) -> anyhow::Result<()> {
        self.digest.clear();
        self.digest = digest::json(self)?;
        Ok(())
    }

    pub fn verify(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.schema_version == SCHEMA_VERSION,
            "unsupported artifact schema"
        );
        anyhow::ensure!(!self.states.is_empty(), "artifact has no states");
        let mut unsigned = self.clone();
        let expected = std::mem::take(&mut unsigned.digest);
        anyhow::ensure!(!expected.is_empty(), "artifact digest is missing");
        anyhow::ensure!(
            digest::json(&unsigned)? == expected,
            "artifact integrity check failed"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Pass,
    Fail,
    Inconclusive,
    PreparationRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub key: String,
    pub line: String,
    pub viewport: u32,
    pub scenario: String,
    pub target: String,
    pub property: String,
    pub source: String,
    pub candidate: String,
    pub severity: String,
    pub confidence: String,
    /// Every element behind a grouped root cause, so the text report can list
    /// them one per line instead of one unreadable comma run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub schema_version: u32,
    pub status: Status,
    pub findings: Vec<Finding>,
    pub suppressed_duplicates: usize,
    pub elapsed_ms: u128,
    pub source_digest: String,
    pub candidate_digest: String,
    pub diagnostic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Findings withheld by a declared allowance, so a genuinely expected
    /// difference does not keep the comparison permanently unclean.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed: Vec<Finding>,
    /// Allowances that matched nothing, so a stale allowance is visible
    /// instead of quietly hiding future findings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unused_allowances: Vec<String>,
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;

    #[test]
    fn empty_extended_evidence_preserves_legacy_artifact_shape() {
        let state = State {
            viewport: Viewport {
                width: 1440,
                height: 900,
            },
            scenario: "base".into(),
            nodes: BTreeMap::from([("target".into(), NodeEvidence::default())]),
            active_element: String::new(),
            runtime: RuntimeEvidence::default(),
            screenshot_sha256: String::new(),
            raster_tiles: Vec::new(),
            capture_complete: true,
        };
        let mut artifact = Artifact {
            schema_version: SCHEMA_VERSION,
            source: SourceIdentity {
                requested_url: "https://source.example".into(),
                rendered_url: "https://source.example".into(),
                browser: "browser".into(),
                fingerprint: "fingerprint".into(),
            },
            actions: Vec::new(),
            states: vec![state],
            digest: String::new(),
        };
        artifact.seal().unwrap();
        let json = serde_json::to_string(&artifact).unwrap();
        assert!(!json.contains("\"motions\""));
        assert!(!json.contains("\"layoutShifts\""));
        assert!(!json.contains("\"rasterKind\""));
        assert!(!json.contains("\"rasterTiles\""));
        serde_json::from_str::<Artifact>(&json)
            .unwrap()
            .verify()
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_corruption_is_rejected() {
        let mut session = Session {
            schema_version: SCHEMA_VERSION,
            side: Side::Source,
            cdp_url: "http://127.0.0.1:1".into(),
            target_id: "target".into(),
            requested_url: "http://source.test".into(),
            rendered_url: "http://source.test".into(),
            browser: "browser".into(),
            executable: "browser.exe".into(),
            profile: "profile".into(),
            viewport: Viewport {
                width: 1440,
                height: 900,
            },
            digest: String::new(),
        };
        session.seal().unwrap();
        session.requested_url.push_str("/corrupt");
        assert!(session.verify().is_err());
    }
}
