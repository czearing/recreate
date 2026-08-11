use crate::model::Animation;
use serde_json::Value;

/// A captured animation states `delay`, `duration` and `endDelay` on its own local timeline,
/// and carries `playbackRate` as the speed that timeline runs at against the wall clock. CSS
/// has no rate: every longhand here is wall-clock, at an implied rate of one. So the rate is
/// not carried across, it is *spent* — it converts every local-time quantity and is then
/// discarded, which is the only way to say the same thing in a vocabulary missing the word.
///
/// Spending it on some quantities and not others is a unit error rather than a rounding
/// difference: two numbers read from one timing object end up denominated differently inside
/// one shorthand. `Timing` therefore reads the rate once and is the only thing that spends
/// it, so a local-time quantity added later cannot reach the stylesheet without saying so.
struct Timing<'a> {
    values: &'a Value,
    /// The magnitude, clamped away from zero. A rate of zero is legal — it stops local time
    /// advancing — and dividing by it yields a non-numeric token, which is not a bad value
    /// but a dropped declaration that takes every longhand beside it in the block.
    rate: f64,
    /// The sign, which no number can express and the direction keyword absorbs instead.
    reversed: bool,
}

impl<'a> Timing<'a> {
    fn new(animation: &'a Animation) -> Self {
        let rate = animation.timing["playbackRate"].as_f64().unwrap_or(1.0);
        Self {
            values: &animation.timing,
            rate: rate.abs().max(f64::EPSILON),
            reversed: rate < 0.0,
        }
    }

    /// The one door a local-time quantity leaves through. The result is emitted at its
    /// honest length; a division that does not come out evenly is still exact, and rounding
    /// it would lay a second error over the one being corrected.
    fn wall_clock(&self, quantity: &str) -> String {
        format!(
            "{}ms",
            self.values[quantity].as_f64().unwrap_or(0.0) / self.rate
        )
    }

    fn keyword(&self, name: &str, fallback: &'static str) -> &str {
        self.values[name].as_str().unwrap_or(fallback)
    }
}

pub fn declarations(animations: &[&Animation], names: &[String]) -> String {
    format!(
        "animation-name:{};animation-duration:{};animation-timing-function:{};\
         animation-delay:{};animation-iteration-count:{};animation-direction:{};\
         animation-fill-mode:{};animation-play-state:{};",
        names.join(","),
        list(animations, duration),
        list(animations, easing),
        list(animations, delay),
        list(animations, iterations),
        list(animations, direction),
        list(animations, fill),
        list(animations, play_state)
    )
}

fn list(animations: &[&Animation], value: fn(&Timing) -> String) -> String {
    animations
        .iter()
        .map(|animation| value(&Timing::new(animation)))
        .collect::<Vec<_>>()
        .join(",")
}

fn duration(timing: &Timing) -> String {
    timing.wall_clock("duration")
}

fn delay(timing: &Timing) -> String {
    timing.wall_clock("delay")
}

fn easing(timing: &Timing) -> String {
    timing.keyword("easing", "linear").into()
}

fn iterations(timing: &Timing) -> String {
    let value = &timing.values["iterations"];
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_f64().map(|number| number.to_string()))
        .unwrap_or_else(|| "1".into())
}

fn direction(timing: &Timing) -> String {
    let direction = timing.keyword("direction", "normal");
    if !timing.reversed {
        return direction.into();
    }
    match direction {
        "normal" => "reverse",
        "reverse" => "normal",
        "alternate" => "alternate-reverse",
        "alternate-reverse" => "alternate",
        _ => direction,
    }
    .into()
}

fn fill(timing: &Timing) -> String {
    match timing.keyword("fill", "none") {
        "auto" => "none",
        fill => fill,
    }
    .into()
}

fn play_state(timing: &Timing) -> String {
    if timing.keyword("playState", "running") == "paused" {
        "paused"
    } else {
        "running"
    }
    .into()
}

#[cfg(test)]
#[path = "animation_timing_tests.rs"]
mod tests;
