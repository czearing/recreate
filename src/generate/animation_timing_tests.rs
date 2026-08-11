use crate::model::Animation;
use serde_json::json;

/// A scripted animation whose timing states everything in the animation's own local time.
/// `duration` and `delay` arrive from the same `getTiming()` call, in the same units.
fn scripted(delay: f64, duration: f64, rate: serde_json::Value) -> Animation {
    Animation {
        target: "html>body>div".into(),
        name: String::new(),
        keyframes: vec![json!({"opacity":"0"}), json!({"opacity":"1"})],
        timing: json!({
            "delay": delay,
            "duration": duration,
            "easing": "linear",
            "iterations": 1,
            "direction": "normal",
            "fill": "both",
            "playState": "running",
            "playbackRate": rate
        }),
    }
}

fn emitted(animation: &Animation, property: &str) -> String {
    let css = super::declarations(&[animation], &["recreateX".into()]);
    css.split(';')
        .find_map(|declaration| declaration.trim().strip_prefix(&format!("{property}:")))
        .unwrap_or_else(|| panic!("{property} was not emitted at all in {css}"))
        .to_string()
}

/// The defect. `playbackRate` is the rate the animation's local time advances against the
/// wall clock, so every local-time quantity is `local / |rate|` in wall-clock terms — and
/// CSS has no `playbackRate`, so the rate must be spent on all of them and then discarded.
/// Spending it on `duration` alone leaves two numbers from one timing object denominated in
/// different units inside one shorthand: at rate 2 the element sits still for a full extra
/// second and only then plays at the right speed.
#[test]
fn scales_every_local_time_quantity_by_the_rate_that_governs_them_all() {
    let animation = scripted(2000.0, 4000.0, json!(2));
    assert_eq!(emitted(&animation, "animation-duration"), "2000ms");
    assert_eq!(
        emitted(&animation, "animation-delay"),
        "1000ms",
        "the delay was copied in local time beside a duration converted to wall-clock time"
    );
}

/// The relation, stated without naming a millisecond. Whatever factor the rate applies to
/// the duration it must apply to the delay, so no formatting or rounding choice can satisfy
/// one assertion while breaking the invariant.
#[test]
fn moves_the_delay_by_the_same_factor_it_moves_the_duration() {
    let scale = |rate: serde_json::Value| {
        let animation = scripted(2000.0, 4000.0, rate);
        let read = |property| {
            emitted(&animation, property)
                .trim_end_matches("ms")
                .parse::<f64>()
                .expect("a numeric millisecond value")
        };
        (
            4000.0 / read("animation-duration"),
            2000.0 / read("animation-delay"),
        )
    };
    for rate in [json!(2), json!(0.5), json!(4)] {
        let (duration_factor, delay_factor) = scale(rate.clone());
        assert_eq!(
            duration_factor, delay_factor,
            "duration and delay disagree about the rate at {rate}"
        );
    }
}

/// The inverse guard, and the reason the repair can be told apart from a churn of the
/// golden output: rate one is the overwhelming majority of captured animations, and for
/// them the conversion must be an exact identity down to the byte.
#[test]
fn leaves_an_animation_playing_at_the_ordinary_rate_untouched() {
    let stated = super::declarations(
        &[&scripted(2000.0, 4000.0, json!(1))],
        &["recreateX".into()],
    );
    let mut absent = scripted(2000.0, 4000.0, json!(1));
    absent
        .timing
        .as_object_mut()
        .unwrap()
        .remove("playbackRate");
    let implied = super::declarations(&[&absent], &["recreateX".into()]);
    assert_eq!(stated, implied, "a stated rate of one is not the default");
    assert!(stated.contains("animation-duration:4000ms"), "{stated}");
    assert!(stated.contains("animation-delay:2000ms"), "{stated}");
}

/// A rate of zero is legal — it stops local time advancing — and dividing by it yields
/// infinity. A non-numeric token does not degrade gracefully: the declaration is dropped as
/// invalid, taking every other timing longhand emitted beside it in the same block, which
/// turns a wrong number into a lost animation. The duration path already clamps; the delay
/// must reuse that clamp rather than grow a second one that disagrees with it.
#[test]
fn never_emits_a_token_no_parser_accepts_for_a_stopped_animation() {
    let css = super::declarations(
        &[&scripted(2000.0, 4000.0, json!(0))],
        &["recreateX".into()],
    );
    for token in ["inf", "NaN", "nan"] {
        assert!(!css.contains(token), "emitted {token} in {css}");
    }
    for property in ["animation-duration", "animation-delay"] {
        let value = emitted(&scripted(2000.0, 4000.0, json!(0)), property);
        assert!(
            value
                .trim_end_matches("ms")
                .parse::<f64>()
                .is_ok_and(f64::is_finite),
            "{property} emitted the non-finite value {value}"
        );
    }
}

/// A negative rate is absorbed in two halves: the magnitude scales the numbers and the sign
/// flips the direction keyword. Both halves must happen, and the delay must follow the
/// magnitude exactly as the duration does rather than being left in local time because the
/// sign was already dealt with.
#[test]
fn absorbs_a_reversed_rate_as_a_magnitude_and_a_direction_together() {
    let animation = scripted(2000.0, 4000.0, json!(-2));
    assert_eq!(emitted(&animation, "animation-duration"), "2000ms");
    assert_eq!(emitted(&animation, "animation-delay"), "1000ms");
    assert_eq!(emitted(&animation, "animation-direction"), "reverse");
}

/// A rate the division cannot express exactly must be emitted at its honest length. Rounding
/// would be a second error laid over the first, and an asymmetric one, since the duration
/// path does not round today.
#[test]
fn does_not_round_a_conversion_that_does_not_divide_evenly() {
    let animation = scripted(1000.0, 1000.0, json!(3));
    let duration = emitted(&animation, "animation-duration");
    assert_eq!(
        emitted(&animation, "animation-delay"),
        duration,
        "two equal local-time values were formatted by different rules"
    );
    assert!(duration.starts_with("333.33"), "rounded away: {duration}");
}

/// The values the rate does not govern must not move. Only quantities measured on the
/// animation's local timeline are rescaled; an iteration count, an easing keyword or a fill
/// mode are not durations and a conversion that touched them would be widening past the
/// invariant rather than satisfying it.
#[test]
fn rescales_only_the_quantities_measured_in_local_time() {
    let css = super::declarations(
        &[&scripted(2000.0, 4000.0, json!(2))],
        &["recreateX".into()],
    );
    assert!(css.contains("animation-iteration-count:1"), "{css}");
    assert!(css.contains("animation-timing-function:linear"), "{css}");
    assert!(css.contains("animation-fill-mode:both"), "{css}");
    assert!(css.contains("animation-play-state:running"), "{css}");
}
