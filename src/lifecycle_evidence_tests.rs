//! Tests for the two rules that decide whether a page still owes the recorder something:
//! what counts as an image still arriving, and what counts as a pause it recovered from.
//! Both are read out of the shipped script under Node, so the assertions constrain the
//! decision that ships rather than a Rust restatement of it.

/// An image the browser has deferred is not work the page is waiting on.
///
/// The recorder held itself busy while any `document.images` entry was not `complete`, which
/// is true from parse until fetch for every image and stays true forever for one the browser
/// has chosen not to fetch at all. A page served from disk has neither kind outstanding for
/// more than a frame, so the clause read as correct; measured on notion.com at one viewport,
/// 14 of 60 images were permanently incomplete and every one was `loading="lazy"` below the
/// fold, holding the recorder to its full 12-second ceiling on every viewport of every run.
///
/// The distinction cannot come from the `loading` attribute, which names one input and would
/// miss the next — a deferred `srcset` candidate, an empty `src`, a decode never started. It
/// comes from the engine: `currentSrc` is empty until the browser has selected a candidate
/// and begun fetching it, so an incomplete image holding one is genuinely in flight, and one
/// without has not been started and may never be.
#[test]
fn an_image_the_browser_has_not_begun_fetching_is_not_waited_for() {
    assert!(
        loading("loaded", &[("https://example.test/hero.png", false)]),
        "an image already being fetched must hold the recorder open"
    );
    assert!(
        !loading("loaded", &[("", false)]),
        "an image the browser never began fetching held the recorder open"
    );
    assert!(
        !loading("loaded", &[("https://example.test/hero.png", true)]),
        "an image that finished held the recorder open"
    );
    assert!(
        loading("loading", &[]),
        "fonts still arriving must hold the recorder open"
    );
    assert!(
        !loading("loaded", &[]),
        "a page with loaded fonts and no images has nothing to wait for"
    );
}

/// Reads the shipped rule against a document double: each image is its `currentSrc` and its
/// `complete` flag, which is everything the rule is allowed to consult.
fn loading(fonts: &str, images: &[(&str, bool)]) -> bool {
    let entries: Vec<String> = images
        .iter()
        .map(|(current_src, complete)| {
            format!("{{ currentSrc: {current_src:?}, complete: {complete} }}")
        })
        .collect();
    let document = format!(
        "{{ fonts: {{ status: {fonts:?} }}, images: [{}] }}",
        entries.join(", ")
    );
    crate::node_eval::json(&format!(
        "{}\nconsole.log(JSON.stringify(lifecycleLoading({document})));",
        crate::lifecycle_settle_script::SOURCE
    ))
    .as_bool()
    .unwrap()
}
/// The silence before a page's first change is not a gap it recovered from.
///
/// Both the recorder and the dynamic observer wait for the page to stay still for longer than
/// the widest pause it has already come back from. Both computed that pause from the load
/// origin, so a page whose first edit lands eight seconds in was credited with an eight-second
/// cadence it had never demonstrated — and then held open waiting to out-sit it, every
/// viewport, every run. A fixture edits itself in the first frame, so the invented gap is
/// always near zero there and the rule reads as correct.
///
/// One rule, so one owner: the observer folds its event stamps through the same function the
/// recorder applies incrementally.
#[test]
fn silence_before_the_first_change_is_not_a_gap() {
    assert_eq!(longest_gap(&[]), 0.0, "no changes can show no cadence");
    assert_eq!(
        longest_gap(&[8000.0]),
        0.0,
        "one change eight seconds in was credited with an eight-second cadence"
    );
    assert_eq!(
        longest_gap(&[8000.0, 8100.0]),
        100.0,
        "the gap between two changes is the only pause the page came back from"
    );
    assert_eq!(
        longest_gap(&[8100.0, 8000.0, 9000.0]),
        900.0,
        "gaps are measured between changes in time order, not arrival order"
    );
}

fn longest_gap(times: &[f64]) -> f64 {
    let list: Vec<String> = times.iter().map(|time| time.to_string()).collect();
    crate::node_eval::evaluate(
        crate::lifecycle_settle_script::SOURCE,
        &format!("lifecycleLongestGap([{}])", list.join(", ")),
    )
    .as_f64()
    .unwrap()
}
