//! What a reading records when an animation has already stopped. The rest of the motion policy
//! is about a value still travelling; this file is about the one that has arrived and stayed.
//!
//! `animation-fill-mode: forwards` keeps the effect applying after the active interval ends, and
//! there is no later moment at which the element reverts. So the filled value is not a frame the
//! clock happened to catch — it is where the page rests, and a reading that holds it out records
//! the underlying value the page never shows. Every closed Drawer in the shipped bundle is this
//! shape: `animation: drawer-exit-start .2s ease forwards` over `@keyframes { to { transform:
//! translateX(-100%) } }`, resting off-screen with nothing in its own declarations saying so.
//!
//! What separates it from the frames is asked of the engine rather than read off a keyword: an
//! effect placed at its own end time either still reports progress there or does not.

use crate::capture_motion_double::evaluate;

/// The defect, stated over the value a reader takes rather than over any emitted stylesheet. The
/// element declares the property nowhere but the keyframes, so holding the fill out does not
/// merely record a different value — it records nothing, and the declaration is pruned as an
/// unauthored default.
#[test]
fn a_fill_that_outlives_its_animation_is_the_value_the_page_rests_at() {
    let seen = evaluate(
        "globalThis.style['transform'] = 'none';\
         \nanimate('closed', 'transform', { to: 'translateX(-100%)', frame: 'translateX(-60%)', \
         fills: true, at: 200, playState: 'finished' });\
         \nconst resume = restingRead(() => { globalThis.during = computed('transform'); });\
         \nglobalThis.afterRead = computed('transform');\
         \nresume();",
        "[globalThis.during, globalThis.afterRead, computed('transform')]",
    );
    assert_eq!(
        seen,
        serde_json::json!([
            "translateX(-100%)",
            "translateX(-100%)",
            "translateX(-100%)"
        ]),
        "the fill is where the page rests, so it is what every reading takes: {seen}"
    );
}

/// The same claim at the position that actually decides the record. The measurement in the middle
/// of a read reverts every element and puts its style attribute back, which cancels the finished
/// animation and starts a fresh one running from the beginning — so by the time any value is read
/// the fill is in the future, not the past. A remedy that only kept an already-finished animation
/// would pass the test above and still record nothing here.
#[test]
fn a_fill_the_read_itself_restarted_is_still_taken_at_its_end() {
    let seen = evaluate(
        "globalThis.style['opacity'] = '1';\
         \nconst resume = restingRead(() => { \
         animate('restarted', 'opacity', { to: '0', frame: '0.5', fills: true }); });\
         \nglobalThis.afterRead = computed('opacity');\
         \nresume();",
        "[globalThis.afterRead, computed('opacity'), globalThis.animations[0].playState]",
    );
    assert_eq!(
        seen,
        serde_json::json!(["0", "0.5", "running"]),
        "read at the end it is going to reach, and handed back still on its way there: {seen}"
    );
}

/// The control that keeps the repair from becoming "record whatever is applying". An animation
/// that lets go at its end applies nothing once it is over, so its last frame is not a resting
/// value and the element belongs on what the cascade produced. This is the scene's enter
/// animation, and it is the case the whole policy was written for.
#[test]
fn an_animation_that_lets_go_at_its_end_contributes_nothing() {
    let seen = evaluate(
        "globalThis.style['transform'] = 'none';\
         \nanimate('enter', 'transform', \
         { to: 'translateX(-100%)', frame: 'translateX(-40%)' });\
         \nconst resume = restingRead(() => { globalThis.during = computed('transform'); });\
         \nglobalThis.afterRead = computed('transform');\
         \nresume();",
        "[globalThis.during, globalThis.afterRead, computed('transform')]",
    );
    assert_eq!(
        seen,
        serde_json::json!(["none", "none", "translateX(-40%)"]),
        "held out of the read and left applying its frame afterwards: {seen}"
    );
}

/// An endless animation is reported with no end time at all rather than an infinite one, so the
/// question "where does this come to rest" has no answer for it and it is held out whole. The
/// value it declares to fill with is beside the point: it will never get there.
#[test]
fn an_animation_that_never_ends_has_no_resting_frame_to_keep() {
    let seen = evaluate(
        "globalThis.style['transform'] = 'none';\
         \nanimate('spin', 'transform', { to: 'rotate(360deg)', frame: 'rotate(90deg)', \
         endless: true, fills: true });\
         \nconst resume = restingRead(() => { globalThis.during = computed('transform'); });\
         \nglobalThis.afterRead = computed('transform');\
         \nresume();",
        "[globalThis.during, globalThis.afterRead, computed('transform')]",
    );
    assert_eq!(
        seen,
        serde_json::json!(["none", "none", "rotate(90deg)"]),
        "an endless fill is still a frame, so it is held out like every other: {seen}"
    );
}

/// Keeping a fill in the read must not be keeping the page's own animation in the read. The
/// recreation goes on animating, which needs that animation, its effect, its place on its own
/// timeline and its play state all still there afterwards — so the fill is presented from
/// somewhere else while the animation itself is held out exactly as before.
#[test]
fn the_animation_the_page_owns_is_untouched_while_its_fill_is_held() {
    let seen = evaluate(
        "const closed = animate('closed', 'transform', { to: 'translateX(-100%)', \
         frame: 'translateX(-60%)', fills: true, at: 200, playState: 'finished' });\
         \nconst effect = closed.effect;\
         \nconst resume = restingRead(() => { globalThis.duringEffect = closed.effect; });\
         \nresume();\
         \nglobalThis.same = closed.effect === effect;",
        "[globalThis.duringEffect, globalThis.same, closed.currentTime, closed.playState, names()]",
    );
    assert_eq!(
        seen,
        serde_json::json!([null, true, 200, "finished", ["closed"]]),
        "the fill was presented elsewhere and the animation was handed back whole: {seen}"
    );
}

/// A read sweeps twice, because the measurement in the middle of it restarts every animation.
/// Whatever presents a fill during the read is itself an animation the page reports, so the
/// second sweep will pick it up and hold it in turn unless it is known for what it is — and then
/// two releases both claim the same effect and the one that runs last keeps it.
#[test]
fn a_fill_already_being_held_is_not_held_a_second_time() {
    let seen = evaluate(
        "const closed = animate('closed', 'transform', { to: 'translateX(-100%)', \
         frame: 'translateX(-60%)', fills: true, at: 200, playState: 'finished' });\
         \nconst resume = restingRead(() => { globalThis.during = globalThis.animations.length; });\
         \nglobalThis.afterRead = globalThis.animations.length;\
         \nresume();",
        "[globalThis.during, globalThis.afterRead, names(), closed.effect !== null]",
    );
    assert_eq!(
        seen,
        serde_json::json!([2, 2, ["closed"], true]),
        "one holder for one fill, however many times the read sweeps: {seen}"
    );
}

/// A fill is only where the page rests if the animation is going to get there. An animation that
/// has been paused before its end never will: it is standing where it stands, and reading it at
/// an end it will not reach would fabricate a value exactly as surely as reading a frame in
/// flight does. So the resting time is the end it is travelling towards while it is running, and
/// its own position when it is not.
#[test]
fn an_animation_that_is_not_going_anywhere_rests_where_it_stands() {
    let seen = evaluate(
        "globalThis.style['opacity'] = '0.11';\
         \nanimate('scrubbed', 'opacity', { to: '0.79', frame: '0.11', fills: true, at: 0, \
         playState: 'paused' });\
         \nconst resume = restingRead(() => { globalThis.during = computed('opacity'); });\
         \nglobalThis.afterRead = computed('opacity');\
         \nresume();",
        "[globalThis.during, globalThis.afterRead, computed('opacity')]",
    );
    assert_eq!(
        seen,
        serde_json::json!(["0.11", "0.11", "0.11"]),
        "a paused animation is at rest already, so its end is not its resting value: {seen}"
    );
}

/// The same statement for an animation travelling the other way. A negative rate is running
/// towards the beginning, so that is where it comes to rest, and a rule that reached for the end
/// because the animation happened to be running would take the frame furthest from it — while
/// one that never looked ahead at all would take wherever it had got to.
#[test]
fn an_animation_running_backwards_rests_at_the_beginning_it_is_heading_for() {
    let seen = evaluate(
        "globalThis.style['opacity'] = '1';\
         \nanimate('reversed', 'opacity', { from: '0.9', frame: '0.5', to: '0.1', fills: true, \
         at: 150, rate: -1 });\
         \nconst resume = restingRead(() => { globalThis.during = computed('opacity'); });\
         \nresume();",
        "globalThis.during",
    );
    assert_eq!(
        seen,
        serde_json::json!("0.9"),
        "read at the beginning it is heading for, not the end it left or the point it is at: \
         {seen}"
    );
}
