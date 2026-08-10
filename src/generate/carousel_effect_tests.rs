//! The shipped carousel effect, driven rather than read.
//!
//! The inference tests next door decide what the generator admits. These decide what the
//! browser does with that decision, which is a separate question: the rule once shipped twice,
//! and the browser copy was the weaker one — it scanned every element for a disabled/enabled
//! pair near overflow, keeping no constraint but the overflow itself, and it ran precisely
//! when the generator had declined to guess.

use super::carousel_inference::EFFECT;

/// Every element the effect touches must come from the decision already made, so the effect
/// cannot reintroduce a search of its own however the generator is changed.
#[test]
fn the_shipped_effect_never_chooses_a_carousel_itself() {
    let queries = EFFECT.match_indices("document.querySelector").count();
    assert_eq!(queries, 3, "{EFFECT}");
    for (index, _) in EFFECT.match_indices("document.querySelector") {
        let argument = &EFFECT[index..];
        let argument = &argument[argument.find('(').unwrap() + 1..];
        assert!(
            argument.starts_with("inferredCarousel."),
            "effect queries the document for something it was not given: {argument:.60}"
        );
    }
    assert!(!EFFECT.contains("querySelectorAll('body *')"), "{EFFECT}");
}

/// The effect is spliced into the app unconditionally, so its own opening guard is the whole
/// reason an app with nothing to replay binds nothing. That makes the guard load-bearing
/// rather than defensive: without it the generator's `null` — which means "a real carousel was
/// captured, do not guess" — would once again be the condition that starts the effect looking.
/// Driven rather than read, because a guard can be present in the text and unreachable.
#[test]
fn the_shipped_effect_binds_nothing_when_no_carousel_was_admitted() {
    assert_eq!(bound_listeners("null"), 0);
    assert_eq!(
        bound_listeners(r#"{previous:'#p',next:'#n',target:'#t',extent:500}"#),
        2,
        "an admitted carousel must still bind, or the guard has swallowed the feature"
    );
}

/// Runs the shipped `EFFECT` against a document double and reports how many click listeners it
/// binds for the given inference.
fn bound_listeners(inferred: &str) -> usize {
    let body = EFFECT
        .strip_prefix("useEffect(()=>{")
        .and_then(|rest| rest.strip_suffix("},[]);"))
        .expect("EFFECT is a useEffect call with no dependencies");
    crate::node_eval::evaluate(
        &format!(
            r#"let bound = 0;
const element = () => ({{
  addEventListener: () => {{ bound++; }},
  removeEventListener: () => {{ bound--; }},
  querySelectorAll: () => [],
  scrollTo: () => {{}}
}});
const nodes = {{ '#p': element(), '#n': element(), '#t': element() }};
const document = {{ querySelector: selector => nodes[selector] ?? null }};
const animateScroll = () => {{}};
const inferredCarousel = {inferred};
(() => {{ {body} }})();
"#
        ),
        "({ bound })",
    )["bound"]
        .as_u64()
        .expect("the double reports a listener count") as usize
}
