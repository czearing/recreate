use super::*;
use serde_json::json;

fn animation(target: &str) -> Animation {
    Animation {
        target: target.into(),
        keyframes: vec![json!({"opacity":"0"}), json!({"opacity":"1"})],
        timing: json!({
            "duration": 400,
            "iterations": "infinite",
            "direction": "normal",
            "fill": "both",
            "playState": "paused",
            "playbackRate": -2
        }),
    }
}

#[test]
fn converts_geometry_frames_to_css() {
    let frame = json!({"offset":0.5,"x":10,"y":20,"opacity":"0.5","easing":"ease-in"});
    let css = declarations(frame.as_object(), (15.0, 30.0));
    assert!(css.contains("translate:-5px -10px"));
    assert!(css.contains("opacity:0.5"));
    assert!(css.contains("animation-timing-function:ease-in"));
}

#[test]
fn preserves_animation_timing_controls() {
    let mut classes = BTreeMap::from([("html>body".into(), "base".into())]);
    let mut css = String::new();
    append(&[animation("html>body")], &mut classes, &mut css);
    assert!(css.contains("animation-duration:200ms"));
    assert!(css.contains("animation-iteration-count:infinite"));
    assert!(css.contains("animation-direction:reverse"));
    assert!(css.contains("animation-fill-mode:both"));
    assert!(css.contains("animation-play-state:paused"));
    assert!(!css.contains("prefers-reduced-motion"));
}

#[test]
fn rejects_sampled_layout_observations() {
    let mut sampled = animation("html");
    sampled.keyframes = vec![
        json!({"offset":0,"opacity":"1","transform":"none","x":0,"y":0,"width":1440,"height":0}),
        json!({"offset":1,"opacity":"1","transform":"none","x":0,"y":0,"width":1440,"height":229}),
    ];
    sampled.timing = json!({"delay":0,"duration":2513.4,"easing":"linear","iterations":1});
    let mut classes = BTreeMap::from([("html".into(), "base".into())]);
    let mut css = String::new();
    append(&[sampled], &mut classes, &mut css);
    assert_eq!(classes["html"], "base");
    assert!(!css.contains("@keyframes"));
}

#[test]
fn preserves_authored_geometry_animations() {
    let mut authored = animation("html>body>aside");
    authored.keyframes = vec![
        json!({"computedOffset":0,"easing":"linear","width":"0px"}),
        json!({"computedOffset":1,"easing":"linear","width":"240px"}),
    ];
    let mut classes = BTreeMap::from([("html>body>aside".into(), "base".into())]);
    let mut css = String::new();
    append(&[authored], &mut classes, &mut css);
    assert!(classes["html>body>aside"].contains(" a"));
    assert!(css.contains("width:240px"));
}

#[test]
fn reuses_identical_animation_output_across_targets() {
    let mut classes = BTreeMap::from([
        ("html>body>div:nth-of-type(1)".into(), "first".into()),
        ("html>body>div:nth-of-type(2)".into(), "second".into()),
    ]);
    let mut css = String::new();
    append(
        &[
            animation("html>body>div:nth-of-type(1)"),
            animation("html>body>div:nth-of-type(2)"),
        ],
        &mut classes,
        &mut css,
    );
    assert_eq!(css.matches("@keyframes").count(), 1);
    let first = classes["html>body>div:nth-of-type(1)"]
        .split_whitespace()
        .last();
    let second = classes["html>body>div:nth-of-type(2)"]
        .split_whitespace()
        .last();
    assert_eq!(first, second);
}

#[test]
fn combines_concurrent_animations_on_one_target() {
    let mut first = animation("html>body");
    first.keyframes = vec![json!({"opacity":"0"}), json!({"opacity":"1"})];
    let mut second = animation("html>body");
    second.keyframes = vec![
        json!({"transform":"translateX(0px)"}),
        json!({"transform":"translateX(20px)"}),
    ];
    second.timing["duration"] = json!(600);
    second.timing["playbackRate"] = json!(1);
    let mut classes = BTreeMap::from([("html>body".into(), "base".into())]);
    let mut css = String::new();
    append(&[first, second], &mut classes, &mut css);
    assert_eq!(classes["html>body"].split_whitespace().count(), 2);
    assert!(css.contains("animation-name:recreate"));
    assert!(css.contains(",recreate"));
    assert!(css.contains("animation-duration:200ms,600ms"));
}

#[test]
fn collapses_frames_with_the_same_css_percentage() {
    let mut animation = animation("html>body");
    animation.keyframes = (0..=200)
        .map(|index| json!({"offset":index as f64 / 200.0,"opacity":index as f64 / 200.0}))
        .collect();
    let mut classes = BTreeMap::from([("html>body".into(), "base".into())]);
    let mut css = String::new();
    append(&[animation], &mut classes, &mut css);
    assert_eq!(css.matches("%{").count(), 101);
}

#[test]
fn merged_percentage_frames_keep_complementary_properties() {
    let mut animation = animation("html>body");
    animation.keyframes = vec![
        json!({"offset":0.001,"opacity":"0"}),
        json!({"offset":0.004,"transform":"none"}),
        json!({"offset":1,"opacity":"1","transform":"none"}),
    ];
    let mut classes = BTreeMap::from([("html>body".into(), "base".into())]);
    let mut css = String::new();
    append(&[animation], &mut classes, &mut css);
    assert!(css.contains("0%{opacity:0;transform:none;}"));
}
