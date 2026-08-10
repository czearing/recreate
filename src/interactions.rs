#[path = "interactions_activate.rs"]
mod interactions_activate;
#[path = "interactions_capture.rs"]
mod interactions_capture;
#[path = "interactions_discovery.rs"]
mod interactions_discovery;
#[path = "interactions_evidence.rs"]
mod interactions_evidence;
#[path = "interactions_graph.rs"]
mod interactions_graph;
#[path = "interactions_runtime.rs"]
mod interactions_runtime;
#[path = "interactions_scope.rs"]
mod interactions_scope;
#[path = "interactions_scripts.rs"]
mod interactions_scripts;

pub use interactions_capture::{CapturedGraph, capture_graph};

#[cfg(test)]
#[path = "interactions_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "interactions_evidence_tests.rs"]
mod evidence_tests;

#[cfg(test)]
#[path = "interactions_tests.rs"]
mod tests;
