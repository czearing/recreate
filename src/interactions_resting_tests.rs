//! Driving the page back to its baseline produces a state that describes the baseline, not the
//! candidate that ran before it, so it is read once and recalled afterwards. The recall is only
//! sound while the witness that stands for "the page is where it was" still matches, and a
//! witness taken at one viewport says nothing about another. These tests hold both conditions.

use super::interactions_runtime::RestingStates;
const SOURCE: &str = include_str!("interactions_runtime.rs");
use crate::model::{PageState, Viewport};

fn state(title: &str, width: u32) -> PageState {
    PageState {
        url: "https://example.test".into(),
        title: title.into(),
        viewport: Viewport {
            width,
            height: 1080,
            dpr: 1.0,
        },
        dom: Default::default(),
        capture_blockers: Vec::new(),
        nodes: Vec::new(),
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}

fn witness(value: &str) -> serde_json::Value {
    serde_json::json!([{ "path": "html>body", "rect": value }])
}

/// The defect: the baseline read was repeated for every candidate, and on a page of any size
/// that read is the whole sweep. A page proven to be back where it was answers from the state
/// already read there.
#[test]
fn recalls_a_state_read_under_the_same_witness() {
    let mut rest = RestingStates::default();
    rest.record((1920, 1080), witness("at-rest"), &state("Home", 1920));
    assert_eq!(
        rest.recall((1920, 1080), &witness("at-rest"))
            .map(|state| state.title.clone()),
        Some("Home".to_string())
    );
}

/// A witness that no longer matches means the page is not the one that was read, so the recorded
/// state does not describe it and reading again is the only correct answer.
#[test]
fn refuses_to_answer_when_the_witness_moved() {
    let mut rest = RestingStates::default();
    rest.record((1920, 1080), witness("at-rest"), &state("Home", 1920));
    assert!(rest.recall((1920, 1080), &witness("menu-open")).is_none());
}

/// Every responsive arm drives the page back at its own size, and the witness carries that
/// size's geometry. Answering across viewports would record one arm's layout under another's.
#[test]
fn refuses_to_answer_across_viewports() {
    let mut rest = RestingStates::default();
    rest.record((1920, 1080), witness("at-rest"), &state("Wide", 1920));
    assert!(rest.recall((640, 1080), &witness("at-rest")).is_none());
    rest.record((640, 1080), witness("at-rest"), &state("Narrow", 640));
    assert_eq!(
        rest.recall((1920, 1080), &witness("at-rest"))
            .map(|state| state.viewport.width),
        Some(1920)
    );
    assert_eq!(
        rest.recall((640, 1080), &witness("at-rest"))
            .map(|state| state.viewport.width),
        Some(640)
    );
}

/// A sweep visits one viewport many times, so the record has to replace rather than accumulate;
/// otherwise a stale entry for the same size survives beside the current one and can be found.
#[test]
fn keeps_one_entry_per_viewport() {
    let mut rest = RestingStates::default();
    rest.record((1920, 1080), witness("first"), &state("First", 1920));
    rest.record((1920, 1080), witness("second"), &state("Second", 1920));
    assert!(rest.recall((1920, 1080), &witness("first")).is_none());
    assert_eq!(
        rest.recall((1920, 1080), &witness("second"))
            .map(|state| state.title.clone()),
        Some("Second".to_string())
    );
}

/// A witness compares attributes, rects and computed values, none of which move while a keyframe
/// runs or a transition reaches rest. Remembering a page that is still changing would freeze one
/// mid-flight frame and replay it into every later state, which is the defect a byte comparison
/// of the corpus caught while every timing budget passed. The condition is asked of the engine,
/// so it holds for CSS animations, CSS transitions and script-driven animations alike.
#[test]
fn asks_the_engine_whether_the_page_is_still_moving_before_remembering_it() {
    assert!(SOURCE.contains("document.getAnimations().length===0"));
    let recorded = SOURCE
        .split("let at_rest")
        .nth(1)
        .expect("the rest condition is taken before the state is read");
    let recall = recorded.find("rest.recall").expect("recall is guarded");
    let record = recorded.find("rest.record").expect("record is guarded");
    let guards: Vec<usize> = recorded
        .match_indices("at_rest")
        .map(|(index, _)| index)
        .collect();
    assert_eq!(guards.len(), 2, "both directions are guarded");
    assert!(guards[0] < recall && guards[1] < record);
}
