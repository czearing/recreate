//! What a reading records when an animation is applying while it is taken. An animation
//! declares its own endpoints in an origin above the cascade, so no frame of it is a value the
//! page rests at, and a property it drives that the author never declared rests at a value no
//! frame ever holds.

use crate::capture_motion_double::evaluate;

/// Where a reader hands the page back, as a statement rather than as a spelling. A call that
/// has been commented out still spells itself, so a check that only looks for the text passes
/// over the one edit that would silently leave every later stage — including the recorder whose
/// whole subject is the motion — reading a page that no longer moves.
fn release_offset(source: &str) -> usize {
    source
        .match_indices("resumeMotion();")
        .find(|(at, _)| {
            source[..*at]
                .rsplit('\n')
                .next()
                .is_some_and(|line| line.trim().is_empty())
        })
        .map(|(at, _)| at)
        .expect("the reader hands the page back once its reading is over")
}

/// Which reader gets which policy. Every settled viewport reading and every reading taken after
/// an interaction is a resting one; the first-paint reading is the only moving one. Each also
/// hands the page back, because a policy that suspends motion and is never released would leave
/// every later stage reading a page that no longer moves.
#[test]
fn each_reader_is_wired_to_the_policy_its_promise_requires() {
    let settled = crate::page_script::source_without_assets();
    let moment = crate::page_script::source_at_first_paint();
    let interaction = crate::interaction_script::source();
    assert!(settled.contains("restingRead(() => measureBaselines"));
    assert!(moment.contains("movingRead(() => measureBaselines"));
    assert!(!moment.contains("restingRead(() => measureBaselines"));
    assert!(interaction.contains("restingRead(() => measureBaselines"));
    for source in [&settled, &moment, &interaction] {
        release_offset(source);
    }
}

/// The measurement in the middle of the read is itself a source of motion: it removes and
/// restores the declarations that own every CSS animation, so the animation held out at the
/// start is gone and a fresh one is running by the time any value is read. A hold that swept
/// once would have let go of the page before the reading that matters.
#[test]
fn motion_the_read_itself_starts_is_held_out_of_everything_read_after_it() {
    let seen = evaluate(
        "globalThis.style['margin-left'] = '10px';\
         \nconst resume = restingRead(() => { \
         new CSSAnimation('restarted', 'margin-left', undefined, false, '40px'); });\
         \nglobalThis.afterRead = computed('margin-left');\
         \nresume();",
        "[globalThis.afterRead, computed('margin-left')]",
    );
    assert_eq!(
        seen,
        serde_json::json!(["10px", "40px"]),
        "held out for the reading that follows the measurement, and put back after it: {seen}"
    );
}

/// The defect, stated over the record rather than over the emitted stylesheet. A property whose
/// authored value disagrees with the frame is read as the frame, and a later stage that happens
/// to recover the authored text hides it — so the claim has to be made where the value is
/// acquired.
#[test]
fn a_value_read_under_a_running_animation_is_the_one_the_page_rests_at() {
    let seen = evaluate(
        "globalThis.style['margin-left'] = '10px';\
         \nnew CSSAnimation('shift', 'margin-left', undefined, false, '40px');\
         \nconst resume = restingRead(() => { globalThis.read = computed('margin-left'); });\
         \nresume();",
        "[globalThis.read, computed('margin-left')]",
    );
    assert_eq!(
        seen,
        serde_json::json!(["10px", "40px"]),
        "the reading is the resting value and the page is left applying its frame: {seen}"
    );
}

/// The production shape, and the one no authored text can rescue: the element declares the
/// property nowhere, so the only value a reading can take is the frame, and recording it
/// fabricates a declaration the author never wrote. Bootstrap's `.spinner-grow` is this case —
/// recorded at `scale(0)`, which under a reduced-motion arm is a permanently invisible element.
#[test]
fn a_property_only_an_animation_gives_a_value_is_read_as_unset() {
    let seen = evaluate(
        "new CSSAnimation('spin', 'transform', undefined, false, 'matrix(0, 1, -1, 0, 0, 0)');\
         \nconst resume = restingRead(() => { globalThis.read = computed('transform'); });\
         \nresume();",
        "globalThis.read === undefined",
    );
    assert_eq!(seen, serde_json::json!(true));
}

/// Held out, not stopped. The recreation has to go on animating, which needs the animation, its
/// effect, its place on its own timeline and its play state all still there afterwards — so a
/// remedy that cancels the motion, or one that declares `animation-name` away and takes the
/// longhands with it, fails here.
#[test]
fn the_page_is_left_animating_exactly_as_it_was_found() {
    let seen = evaluate(
        "const running = new CSSAnimation('spin', 'transform', undefined, false, 'rotate(90deg)');\
         \nconst effect = running.effect;\
         \nconst resume = restingRead(() => { globalThis.duringEffect = running.effect; });\
         \nresume();\
         \nglobalThis.same = running.effect === effect;",
        "[globalThis.duringEffect, globalThis.same, names(), globalThis.running[0].playState]",
    );
    assert_eq!(
        seen,
        serde_json::json!([null, true, ["spin"], "running"]),
        "held out for the read and put back the same: {seen}"
    );
}

/// Not the style sheet's animations — every animation. A remedy expressed as a declaration
/// reaches only what the CSS owns, so an animation a script started goes on applying its frame
/// over the reading; the handle used here is the one every animation has whatever started it.
#[test]
fn motion_no_style_sheet_owns_is_held_out_too() {
    let seen = evaluate(
        "globalThis.style['letter-spacing'] = 'normal';\
         \nnew Animation('scripted', 'letter-spacing', undefined, false, '77px');\
         \nconst resume = restingRead(() => { globalThis.read = computed('letter-spacing'); });\
         \nresume();",
        "[globalThis.read, computed('letter-spacing')]",
    );
    assert_eq!(seen, serde_json::json!(["normal", "77px"]));
}

/// The other half of the one rule, and the control that proves the repair was not achieved by
/// holding out all motion alike. A transition's end is the value the cascade produced, so it is
/// brought to that end and left there — never detached, never put back mid-flight.
#[test]
fn a_transition_is_brought_to_its_end_rather_than_held_out() {
    let seen = evaluate(
        "new CSSTransition('paint', 'color', 'rgb(0, 0, 255)', false, 'rgb(9, 9, 9)');\
         \nconst resume = restingRead(() => { globalThis.duringNames = names(); });\
         \nresume();",
        "[globalThis.duringNames, globalThis.style, names()]",
    );
    assert_eq!(
        seen,
        serde_json::json!([["paint"], { "color": "rgb(0, 0, 255)" }, []]),
        "the transition is still in the page during the read and arrives after it: {seen}"
    );
}

/// The hold covers the whole reading, not one pass of it. Every value a capture records is read
/// from the page, so a release placed before the walk leaves most of the reading taken while the
/// motion applies; and it must come before the motion is itself recorded, or the reader whose
/// subject is how the page moves finds a page with no motion left to describe.
#[test]
fn the_page_is_handed_back_after_the_reading_and_before_the_motion_is_recorded() {
    let source = crate::page_script::source_without_assets();
    let walk = source.find("walk(document.documentElement)").unwrap();
    let release = release_offset(&source);
    let recorded = source.find("const liveAnimations").unwrap();
    assert!(
        walk < release && release < recorded,
        "release at {release} sits between the walk at {walk} and the record at {recorded}"
    );
}

/// An animation the platform has already left without an effect is passed over rather than
/// recorded, so the release puts back only what it took. Without this the suspension would
/// invent an entry whose restoration writes `null` onto an animation it never touched.
#[test]
fn an_animation_that_carries_no_effect_is_passed_over_rather_than_recorded() {
    let seen = evaluate(
        "const bare = new CSSAnimation('bare', 'opacity', undefined, false, '0.5');\
         \nbare.effect = null;\
         \nconst resume = restingRead(() => {});\
         \nresume();",
        "[globalThis.running[0].effect, names()]",
    );
    assert_eq!(seen, serde_json::json!([null, ["bare"]]), "{seen}");
}
