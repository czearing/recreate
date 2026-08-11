use super::fixture::*;
use serde_json::Value;

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
/// recorder must stay until the repeat arrives — here on the fourth change — and still let
/// go long before the ceiling once the page has gone quiet for longer than its own longest
/// gap.
///
/// This used to assert the recorder outlasted the whole scripted scene, which is a claim
/// about the fixture rather than about the rule, and a claim no page that cycles forever can
/// satisfy: measured against a live `setInterval` it cost the full 12s ceiling and a 14.85s
/// capture. The proof point is what the rule names, so that is what is asserted.
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
        elapsed > 30 * FRAME_MS,
        "released before the fourth change proved the cycle"
    );
    assert!(
        elapsed < CEILING_MS,
        "a proven cycle still paid the ceiling"
    );
}

/// The defect the wiring exposed. A page driven by `setInterval` never stops changing, so
/// counting every recorded change as a reason to keep watching left it un-quiet at every
/// instant and released it only at the ceiling. Changes belonging to a group that has
/// already proven its cycle are motion the recorder has described, not information it
/// lacks, so they must not hold it open.
#[test]
fn a_cycle_that_never_stops_is_still_released_once_it_is_proven() {
    let target = "html>div:nth-of-type(1)";
    let rotation = ["Alpha", "Bravo", "Charlie"];
    let scene = (0..900)
        .map(|index| {
            if index % 10 == 0 {
                vec![change(target, rotation[(index / 10) % rotation.len()])]
            } else {
                vec![]
            }
        })
        .collect();
    let elapsed = observed(scene);
    assert!(
        elapsed < CEILING_MS,
        "a page cycling forever was watched to the ceiling"
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

/// The cost of treating an unproven group as a promise. A progression of three values that
/// came to an end has proven no cycle and never will, so counting it as unfinished held the
/// recorder to the full ceiling: measured on a real scene it turned a 2.7s capture into a
/// 16.7s one. Absence of proof is not evidence, so the page's own quiet ends it.
#[test]
fn a_progression_that_came_to_an_end_releases_the_recorder_once_the_page_is_quiet() {
    let target = "html>div:nth-of-type(1)";
    let mut scene: Vec<Vec<Value>> = Vec::new();
    for value in ["Draft", "Reviewing", "Final"] {
        scene.push(vec![change(target, value)]);
        scene.extend(quiet(18));
    }
    scene.extend(quiet(900));
    let elapsed = observed(scene);
    assert!(
        elapsed < CEILING_MS,
        "a progression that stopped was watched to the ceiling"
    );
}

/// Equality is not proof of silence. A page that keeps writing hands the observer a longest
/// gap that is the length of one of its own steps, so a window of exactly that width is a
/// race against the next step — and the observer attaches partway into a gap, so it starts
/// the race already behind. Measured on a live 300ms interval it lost: the run was cut off
/// after four values and the consumer, which folds a cycle only out of whole rounds, emitted
/// a genuine three-value loop as a one-shot progression of four.
#[test]
fn a_cycle_is_watched_until_the_consumer_can_see_it_repeat() {
    let target = "html>div:nth-of-type(1)";
    let history = vec![
        recorded(target, "Alpha", 300),
        recorded(target, "Bravo", 600),
        recorded(target, "Charlie", 900),
    ];
    // A live step slightly wider than the ones already recorded, which is what any real timer
    // produces and what leaves room for a window of exactly the recorded width to expire.
    let step = 20;
    let mut scene: Vec<Vec<Value>> = Vec::new();
    for value in ["Alpha", "Bravo", "Charlie"] {
        scene.extend(quiet(step - 1));
        scene.push(vec![change(target, value)]);
    }
    scene.extend(quiet(900));
    // The third live value completes the second round and is the first that proves the cycle.
    let proof = 3 * step as u64 * FRAME_MS;
    assert!(
        observed_after(history, scene) >= proof,
        "a steady cadence was cut off before its cycle could repeat"
    );
}

/// A page that has changed only once has shown no cadence at all, so its single observed gap
/// must not be read as the whole truth about how fast it moves.
#[test]
fn a_page_that_has_changed_once_is_not_finished_on_its_own_first_gap() {
    let target = "html>div:nth-of-type(1)";
    let mut scene: Vec<Vec<Value>> = vec![vec![change(target, "first")]];
    scene.extend(quiet(900));
    assert!(
        observed(scene) >= crate::attribute_sequence_script::STABLE_GAP_MS as u64,
        "a page that changed once was declared finished on its own first gap"
    );
}
