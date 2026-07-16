use crate::model::Animation;

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

fn list(animations: &[&Animation], value: fn(&Animation) -> String) -> String {
    animations
        .iter()
        .map(|animation| value(animation))
        .collect::<Vec<_>>()
        .join(",")
}

fn duration(animation: &Animation) -> String {
    let rate = animation.timing["playbackRate"]
        .as_f64()
        .unwrap_or(1.0)
        .abs()
        .max(f64::EPSILON);
    format!(
        "{}ms",
        animation.timing["duration"].as_f64().unwrap_or(0.0) / rate
    )
}

fn easing(animation: &Animation) -> String {
    animation.timing["easing"]
        .as_str()
        .unwrap_or("linear")
        .into()
}

fn delay(animation: &Animation) -> String {
    format!("{}ms", animation.timing["delay"].as_f64().unwrap_or(0.0))
}

fn iterations(animation: &Animation) -> String {
    let value = &animation.timing["iterations"];
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_f64().map(|number| number.to_string()))
        .unwrap_or_else(|| "1".into())
}

fn direction(animation: &Animation) -> String {
    let timing = &animation.timing;
    let direction = timing["direction"].as_str().unwrap_or("normal");
    if timing["playbackRate"].as_f64().unwrap_or(1.0) >= 0.0 {
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

fn fill(animation: &Animation) -> String {
    match animation.timing["fill"].as_str() {
        Some("auto") | None => "none",
        Some(fill) => fill,
    }
    .into()
}

fn play_state(animation: &Animation) -> String {
    if animation.timing["playState"].as_str() == Some("paused") {
        "paused"
    } else {
        "running"
    }
    .into()
}
