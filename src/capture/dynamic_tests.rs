use crate::node_eval;
use serde_json::{Value, json};

const HARNESS: &str = include_str!("dynamic_harness.js");
/// The horizon the settle rule owns, past which any page is released.
const CEILING_MS: u64 = 12_000;
/// One virtual animation frame, matching the harness clock.
const FRAME_MS: u64 = 16;

/// One attribute change on one element, as the lifecycle recorder writes it.
fn change(target: &str, value: &str) -> Value {
    json!({ "target": target, "attribute": "title", "value": value })
}

/// A frame that appends nothing, so the page is simply still.
fn quiet(count: usize) -> Vec<Vec<Value>> {
    vec![vec![]; count]
}

/// Runs the real shipped script over a scripted timeline and reports the virtual
/// milliseconds it watched the page for before letting go.
fn observed(scene: Vec<Vec<Value>>) -> u64 {
    let script = HARNESS
        .replace("__SCENE__", &serde_json::to_string(&scene).unwrap())
        .replace("__SCRIPT__", super::source().trim());
    node_eval::json(&script)["elapsed"].as_u64().unwrap()
}

/// The defect this rewrite removes. A page with no attribute behaviour at all used to be
/// held for a four-second floor before anything could look at it, and the floor was charged
/// to every capture of every page. Nothing is pending and nothing has changed, so there is
/// no gap to outwait and the page is released as soon as one frame has confirmed it.
#[test]
fn a_page_with_no_attribute_behaviour_is_released_within_a_frame_or_two() {
    assert!(
        observed(quiet(400)) <= 48,
        "a page that never changed an attribute was still watched"
    );
}

/// Direct evidence outranks quiet. A group whose values keep taking new shapes has not yet
/// proven a cycle, so it is unfinished however long the recorder has run, and only the
/// ceiling releases it.
#[test]
fn a_sequence_that_never_repeats_is_watched_to_the_ceiling() {
    let scene = (0..900)
        .map(|index| vec![change("html>div:nth-of-type(1)", &format!("v{index}"))])
        .collect();
    assert!(observed(scene) >= CEILING_MS);
}

/// The point of watching at all. A cycle is only proven once the values repeat, so the
/// recorder must stay past the first two readings and still let go long before the ceiling
/// once the page has gone quiet for longer than its own longest gap.
#[test]
fn a_proven_cycle_releases_the_recorder_well_before_the_ceiling() {
    let target = "html>div:nth-of-type(1)";
    let mut scene: Vec<Vec<Value>> = Vec::new();
    for index in 0..6 {
        scene.push(vec![change(
            target,
            if index % 2 == 0 { "on" } else { "off" },
        )]);
        scene.extend(quiet(9));
    }
    scene.extend(quiet(900));
    let elapsed = observed(scene);
    assert!(
        elapsed > 60 * FRAME_MS,
        "released before the cycle was shown"
    );
    assert!(
        elapsed < CEILING_MS,
        "a proven cycle still paid the ceiling"
    );
}

/// The wait after the motion stops is measured, not fixed: a page whose changes arrived in
/// wide gaps must be given a wider gap before it counts as finished than one whose changes
/// arrived close together.
#[test]
fn a_page_with_wider_gaps_between_changes_is_watched_for_longer() {
    let target = "html>div:nth-of-type(1)";
    let cycle = |spacing: usize| {
        let mut scene: Vec<Vec<Value>> = Vec::new();
        for index in 0..6 {
            scene.push(vec![change(
                target,
                if index % 2 == 0 { "on" } else { "off" },
            )]);
            scene.extend(quiet(spacing));
        }
        scene.extend(quiet(900));
        scene
    };
    let tight = observed(cycle(2));
    let wide = observed(cycle(20));
    assert!(
        wide > tight,
        "a page with {wide}ms of history was released as fast as one with {tight}ms"
    );
}

/// The whole reason this module no longer decides for itself: the rule it applies is the one
/// `lifecycle_settle_script` owns, inlined rather than restated, so the two cannot drift.
#[test]
fn the_script_carries_the_owning_modules_rule_rather_than_a_copy() {
    let source = super::source();
    assert!(source.contains(crate::lifecycle_settle_script::SOURCE));
    assert!(!source.contains("__LIFECYCLE_SETTLE__"));
    assert!(source.contains("lifecycleSettled("));
}

/// Grouping must stay identical to what the sequence capture applies afterwards, or the
/// recorder would judge a cycle proven that the consumer then reads as unfinished.
#[test]
fn the_reading_groups_changes_the_way_sequence_capture_does() {
    let source = super::source();
    assert!(source.contains("__recreateAttributeMutations"));
    assert!(source.contains("values.at(-1) !== event.value"));
    assert!(source.contains("value === values[index % size]"));
    assert!(source.contains("values.length < 3 || cycle === values.length"));
}
