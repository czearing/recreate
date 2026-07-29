#[path = "interactions_capture.rs"]
mod interactions_capture;
#[path = "interactions_discovery.rs"]
mod interactions_discovery;
#[path = "interactions_evidence.rs"]
mod interactions_evidence;
#[path = "interactions_focus.rs"]
mod interactions_focus;
#[path = "interactions_graph.rs"]
mod interactions_graph;
#[path = "interactions_hover.rs"]
mod interactions_hover;
#[path = "interactions_runtime.rs"]
mod interactions_runtime;
#[path = "interactions_scripts.rs"]
mod interactions_scripts;

#[allow(unused_imports)]
pub use interactions_capture::{CapturedGraph, capture, capture_graph};
pub use interactions_evidence::deduplicate;

#[cfg(test)]
#[path = "interactions_tests.rs"]
mod tests;
