use serde_json::Value;

/// Evaluates the real predicate under Node, so the assertions below constrain the shipped
/// decision rather than the words it is written with.
fn settled(cases: &[(u64, u64, bool)]) -> Vec<bool> {
    let calls = cases
        .iter()
        .map(|(elapsed, since_change, busy)| {
            format!("lifecycleSettled({elapsed}, {since_change}, {busy})")
        })
        .collect::<Vec<_>>()
        .join(",");
    let script = format!("{}\nconsole.log(JSON.stringify([{calls}]));", super::SOURCE);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settle.js");
    std::fs::write(&path, script).unwrap();
    let output = std::process::Command::new("node")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_bool().unwrap())
        .collect()
}

/// The cost of a capture must come from the page, not from a constant. A page that stops
/// moving immediately must release the recorder in about the quiet period, not in twelve
/// seconds.
#[test]
fn a_page_that_stops_moving_releases_the_recorder_before_the_ceiling() {
    let results = settled(&[(1_100, 1_100, false), (11_000, 1_000, false)]);
    assert_eq!(results, vec![true, true]);
}

/// The inverse. A recorder that settles while the page still moves truncates the very
/// motion it exists to record, so every reason a page is still moving must hold it open.
#[test]
fn motion_and_loading_and_recent_change_each_hold_the_recorder_open() {
    let results = settled(&[
        (5_000, 5_000, true),
        (5_000, 100, false),
        (200, 200, false),
        (11_999, 999, true),
    ]);
    assert_eq!(results, vec![false, false, false, false]);
}

/// A page that never stops moving must still be captured, so the horizon stays a ceiling.
#[test]
fn a_page_that_never_settles_is_released_at_the_ceiling() {
    assert_eq!(settled(&[(12_000, 0, true)]), vec![true]);
}
