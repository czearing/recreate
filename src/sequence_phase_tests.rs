//! Where a replayed value progression resumes, driven against the shipped runtime.
//!
//! Phase — which point of a cycle the page was at when it was captured — used to be
//! recovered twice. `runtime/sequence.mjs` reads it back off the element, which works for
//! every kind of value at once because the element is what holds it. The generator also
//! rotated the emitted steps until the captured value came first, and knew how to do that
//! for `textContent` alone, so the two answers agreed for text and disagreed for every
//! attribute the recorder watches: one tick writing one string to both channels serialised
//! as two different arrays.
//!
//! A list of names in the generator could only ever fall behind the list of names the
//! recorder watches, so the generator's copy was removed rather than widened. These tests
//! pin the surviving answer as a general one, stated over the kind of value rather than over
//! any name, which is what lets the watch list grow without this file changing.

use crate::sequence_replay::{cycle, replay};
use serde_json::{Value, json};

/// The subject. An attribute cycle caught mid-loop resumes where it was caught, and the
/// element is never repainted to the first value ever observed.
#[test]
fn an_attribute_cycle_resumes_from_the_captured_value() {
    let result = replay(&cycle("aria-label"), "aria-label", "'Bravo'", 3);
    assert_eq!(
        result["seen"],
        json!(["Bravo", "Charlie", "Alpha", "Bravo"])
    );
}

/// The positive control and the whole comparison. One tick of the page wrote one string to
/// both channels, so a replay that recovers phase from the value rather than from its kind
/// must produce the identical run for both.
#[test]
fn text_and_attribute_channels_written_together_resume_together() {
    let text = replay(&cycle("textContent"), "textContent", "'Bravo'", 4);
    let label = replay(&cycle("aria-label"), "aria-label", "'Bravo'", 4);
    assert_eq!(text["seen"], label["seen"]);
    assert_eq!(
        text["seen"],
        json!(["Bravo", "Charlie", "Alpha", "Bravo", "Charlie"])
    );
}

/// The inverse guard. `data-tooltip` is in no list anywhere in the tool — not in the
/// recorder's watch set, not in the replay — so it stands for the next attribute someone
/// adds. Phasing it proves the rule is about the value and not about the name, which is the
/// property a widened `if` would not have had.
#[test]
fn an_attribute_the_replay_has_no_name_for_is_phased_all_the_same() {
    let result = replay(&cycle("data-tooltip"), "data-tooltip", "'Charlie'", 2);
    assert_eq!(result["seen"], json!(["Charlie", "Alpha", "Bravo"]));
}

/// A value the capture never recorded leaves the phase unknown, and unknown must mean "start
/// at the beginning", never "guess from somewhere else". Reading the IDL property when the
/// attribute lookup fails would be right for four names and wrong for `value`, which is the
/// hardest kind of wrong to notice.
#[test]
fn an_unrecognised_captured_value_starts_the_progression_at_its_beginning() {
    let result = replay(&cycle("aria-label"), "aria-label", "'Delta'", 1);
    assert_eq!(result["seen"], json!(["Delta", "Alpha", "Bravo"]));
}

/// The guard that must survive. Rotation is only sound for a progression the capture watched
/// come back round; a one-shot resumed by wrapping would move the values observed BEFORE the
/// capture to after it. The termination fact still reaches the runtime, whatever kind of
/// value the progression writes.
#[test]
fn an_attribute_progression_recorded_as_finished_rests_where_capture_caught_it() {
    let one_shot = r#"{
      attribute: 'title', repeats: false,
      steps: [
        { value: 'Draft', delay_ms: 300 },
        { value: 'Reviewing', delay_ms: 300 },
        { value: 'Final', delay_ms: 300 }
      ]
    }"#;
    let result = replay(one_shot, "title", "'Final'", 12);
    assert_eq!(result["seen"], json!(["Final"]));
    assert_eq!(result["stopped"], Value::Bool(true));
}
