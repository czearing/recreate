use crate::node_eval;
use serde_json::{Value, json};

/// One animation as the recorder sees it, reduced to the facts the rule reads: where the
/// motion is recorded, whether it is still playing, and how far through its period it is.
struct Running {
    play_state: &'static str,
    delay: u64,
    duration: u64,
    local_time: u64,
    declared: bool,
}

fn running(play_state: &'static str, delay: u64, duration: u64, local_time: u64) -> Running {
    Running {
        play_state,
        delay,
        duration,
        local_time,
        declared: false,
    }
}

/// The same animation, but one a stylesheet declared rather than a script built.
fn declared(duration: u64, local_time: u64) -> Running {
    Running {
        declared: true,
        ..running("running", 0, duration, local_time)
    }
}

fn list(animations: &[Running]) -> String {
    let entries = animations
        .iter()
        .map(|entry| {
            format!(
                "{{target:'t{}',playState:'{}',delay:{},duration:{},localTime:{},declared:{}}}",
                entry.local_time,
                entry.play_state,
                entry.delay,
                entry.duration,
                entry.local_time,
                entry.declared
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{entries}]")
}

/// Evaluates the shipped `lifecycleBusy` against a set of running animations.
fn busy(animations: &[Running], loading: bool) -> bool {
    let call = format!("lifecycleBusy({}, {loading})", list(animations));
    node_eval::evaluate(super::SOURCE, &call).as_bool().unwrap()
}

/// Evaluates the shipped `observedTargets`, reporting the names it considers described.
fn described(animations: &[Running]) -> Value {
    let call = format!("[...observedTargets({})].sort()", list(animations));
    node_eval::evaluate(super::SOURCE, &call)
}

/// The defect this rule exists to remove. An animation that repeats forever is still
/// running at every instant the recorder ever asks, so counting running animations charged
/// every such page the full ceiling. One period describes a periodic process completely, so
/// once a full iteration has been watched there is nothing further to learn by waiting.
#[test]
fn an_endlessly_repeating_animation_stops_counting_as_busy_after_one_period() {
    assert!(busy(&[running("running", 0, 600, 599)], false));
    assert!(!busy(&[running("running", 0, 600, 600)], false));
    assert!(!busy(&[running("running", 0, 600, 4_200)], false));
}

/// A delay is time before the motion starts, so the period to watch begins after it.
#[test]
fn a_delayed_animation_is_watched_for_its_delay_and_then_its_period() {
    assert!(busy(&[running("running", 300, 600, 899)], false));
    assert!(!busy(&[running("running", 300, 600, 900)], false));
}

/// Busy is evidence that something is still to come, so any one unobserved animation is
/// enough, and a finished or never-started animation is never evidence at all.
#[test]
fn one_unobserved_animation_is_enough_and_settled_ones_count_for_nothing() {
    assert!(busy(
        &[
            running("running", 0, 600, 900),
            running("running", 0, 600, 100)
        ],
        false
    ));
    assert!(!busy(&[running("finished", 0, 600, 100)], false));
    assert!(!busy(&[running("idle", 0, 600, 100)], false));
    assert!(!busy(&[], false));
}

/// A pending load is unbounded evidence rather than a period, so it holds on its own.
#[test]
fn a_pending_load_holds_the_recorder_open_with_no_animation_running() {
    assert!(busy(&[], true));
    assert!(busy(&[running("running", 0, 600, 900)], true));
}

/// An animation with no measurable period has no period to wait out.
#[test]
fn an_animation_with_no_duration_has_nothing_left_to_observe() {
    assert!(!busy(&[running("running", 0, 0, 0)], false));
}

/// Motion a stylesheet declares is already written down where the capture reads it, so
/// watching a four-second authored loop for four seconds buys nothing. A script-built
/// animation of the same shape is the opposite case and must still be watched, which is what
/// makes this a statement about where the motion is recorded and not about its length.
#[test]
fn motion_a_stylesheet_declares_is_never_waited_for() {
    assert!(!busy(&[declared(4_000, 0)], false));
    assert!(!busy(&[declared(4_000, 16)], false));
    assert!(busy(&[running("running", 0, 4_000, 16)], false));
}

/// The second half of the same expiry, and the reason a looping page used to run to the
/// ceiling even once it stopped counting as busy: it never stops moving, so the gap since
/// its last change never grows. Motion of an animation already watched through a full period
/// is motion the recorder has described, so its target stops counting as news — while an
/// element moving for any other reason must keep holding the recorder open.
#[test]
fn only_targets_of_fully_observed_animations_stop_counting_as_news() {
    let seen = described(&[
        running("running", 0, 600, 4_200),
        running("running", 0, 600, 100),
        running("finished", 0, 600, 601),
    ]);
    assert_eq!(seen, json!(["t4200", "t601"]));
}
