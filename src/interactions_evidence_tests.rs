//! The evidence hash the capture compares an activation against, executed against a document
//! double so the rule can be constrained without a browser.

use super::{
    interactions_evidence::{candidate_relevant, popup_state},
    interactions_scripts::{Candidate, PREFLIGHT},
    test_support::{empty_node, empty_state},
};

/// A document of `count` buttons where the one at `focused` carries a focus ring, expressed
/// the way a real stylesheet expresses one.
fn document(count: usize, focused: Option<usize>) -> String {
    let ring = focused.map_or("null".to_string(), |index| index.to_string());
    format!(
        r#"
const FOCUSED = {ring};
const elements = Array.from({{ length: {count} }}, (unused, index) => ({{
  tagName: 'BUTTON',
  childElementCount: 0,
  scrollLeft: 0,
  scrollTop: 0,
  getBoundingClientRect: () => ({{ x: index * 40, y: 0, width: 32, height: 32 }}),
  getAttribute: () => null,
  ring: index === FOCUSED
}}));
globalThis.document = {{
  documentElement: {{ scrollWidth: 200, scrollHeight: 100 }},
  querySelectorAll: () => elements
}};
globalThis.getComputedStyle = element => ({{
  getPropertyValue: name => {{
    if (name === 'outline-style') return element.ring ? 'auto' : 'none';
    if (name === 'outline-width') return element.ring ? '2px' : '0px';
    if (name === 'outline-color') return element.ring ? 'rgb(0, 95, 204)' : 'rgb(0, 0, 0)';
    return '';
  }}
}});
"#
    )
}

fn preflight(count: usize, focused: Option<usize>) -> String {
    crate::node_eval::evaluate(&document(count, focused), PREFLIGHT)
        .as_str()
        .expect("preflight returns a string")
        .to_string()
}

/// The premise of the aim-then-measure split. Focus alone moves the evidence hash, so a
/// baseline taken before the probe focused its trigger would differ from the result on every
/// focusable control and report an interaction the page does not have.
///
/// This is also the guard against fixing that the wrong way: dropping the focus properties
/// from the hash would make it stop seeing focus-driven state the page really does have.
#[test]
fn focus_alone_moves_the_evidence_hash() {
    let unfocused = preflight(3, None);
    assert_ne!(
        unfocused,
        preflight(3, Some(0)),
        "focusing a control must change the hash, or focus-driven state is invisible"
    );
    assert_ne!(
        preflight(3, Some(0)),
        preflight(3, Some(1)),
        "the hash must distinguish which control is focused"
    );
    assert_eq!(
        unfocused,
        preflight(3, None),
        "the hash must be a pure function of the page"
    );
}

/// The hash is what decides whether an activation is recorded at all, so it has to answer for
/// the whole page rather than for the trigger. A change anywhere must move it.
#[test]
fn the_evidence_hash_answers_for_the_whole_page() {
    assert_ne!(
        preflight(3, None),
        preflight(4, None),
        "an element appearing must change the hash"
    );
}

#[test]
fn state_controls_remain_relevant_in_successor_states() {
    let state = empty_state();
    let candidate = Candidate {
        path: "html>body>button".into(),
        tag: "button".into(),
        label: "List view".into(),
        occurrence: 0,
        disabled: false,
        navigates: false,
        state_control: true,
    };
    assert!(candidate_relevant(&candidate, &state, &state, None, false));
}

#[test]
fn popup_states_do_not_cross_product_unrelated_state_controls() {
    let mut baseline = empty_state();
    let mut trigger = empty_node("html>body>button:nth-of-type(1)");
    trigger.attributes.insert("role".into(), "tab".into());
    baseline.nodes.push(trigger.clone());
    let mut popup = empty_node("html>body>div:nth-of-type(2)");
    popup.attributes.insert("role".into(), "menu".into());
    let mut reached = baseline.clone();
    reached.nodes.push(popup);
    let candidate = Candidate {
        path: trigger.path.clone(),
        tag: "button".into(),
        label: "Grid".into(),
        occurrence: 0,
        disabled: false,
        navigates: false,
        state_control: true,
    };
    assert!(popup_state(&reached, &baseline));
    assert!(!candidate_relevant(
        &candidate,
        &baseline,
        &reached,
        Some("html>body>button:nth-of-type(2)"),
        true
    ));
}

#[test]
fn repeated_collection_actions_share_a_semantic_family() {
    assert_eq!(
        super::interactions_evidence::action_family("Change icon for Project Alpha"),
        Some("Change icon")
    );
    assert_eq!(
        super::interactions_evidence::action_family("Create notebook"),
        None
    );
}
