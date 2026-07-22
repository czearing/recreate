use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Report {
    pub status: String,
    pub certified: bool,
    pub accuracy_ppm: u32,
    pub qualified_coverage_ppm: u32,
    pub environment_digest: String,
    pub source_artifact_digest: String,
    pub domain_scores_ppm: BTreeMap<String, u32>,
    pub matched_domains: usize,
    pub total_domains: usize,
    pub first_difference: Option<Difference>,
    pub differences: Vec<Difference>,
    pub blockers: Vec<String>,
    pub elapsed_ms: u128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Difference {
    pub scenario: String,
    pub step: usize,
    pub viewport_width: u32,
    pub domain: String,
    pub path: String,
    pub expected: serde_json::Value,
    pub actual: serde_json::Value,
}
