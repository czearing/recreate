//! What every node record must carry, asserted against both producers at once.
//!
//! A capture has two producers of node records — the resting capture and the interaction
//! capture — and a field added to one and forgotten in the other makes the recreation
//! change by state for no authored reason, which nothing downstream can attribute. Every
//! pin here therefore loops over both scripts rather than naming one.
//!
//! Each field below has the same shape of justification: the attribute map and the
//! reflecting DOM property cannot answer the question, so the answer is taken from the
//! engine and recorded once.

use super::source_with_sheets;

/// The capture script as the page receives it when it can read its own stylesheets.
fn source() -> String {
    source_with_sheets(&[])
}

/// Both node-record producers must ask the engine for the disabled state rather than
/// re-derive it, and both must be kept in step: a control disabled by an ancestor
/// `<fieldset>` carries no attribute of its own, and the `disabled` DOM property only
/// reflects that absent attribute, so either substitute answers `false` for it.
#[test]
fn records_the_engine_answered_disabled_state_in_every_node_record() {
    for script in [source(), crate::interaction_script::source()] {
        assert!(script.contains("disabled: element.matches(':disabled')"));
        assert!(!script.contains("disabled: element.disabled"));
    }
}

/// Both node-record producers must record the live state of the IDL attributes that do not
/// reflect a content attribute, and both must be kept in step. For a form control the
/// content attribute is the default — `value` is `defaultValue`, `checked` is
/// `defaultChecked` — so a record built from `element.attributes` carries what the markup
/// authored and never what the page currently shows. Each live read is paired with the
/// engine's own `default*` twin rather than with a hand-derived baseline, which is what
/// keeps a `<textarea>`, whose default is its child text and not an attribute, from needing
/// a case of its own.
#[test]
fn records_the_live_control_state_in_every_node_record() {
    for script in [source(), crate::interaction_script::source()] {
        assert!(script.contains("control_state: recreateControlState(element)"));
        for pair in [
            "live: e => e.value, base: e => e.defaultValue",
            "live: e => e.checked, base: e => e.defaultChecked",
            "live: e => e.selected, base: e => e.defaultSelected",
        ] {
            assert!(script.contains(pair), "missing live/default pair: {pair}");
        }
        // A checkbox or radio holds checkedness; its `value` is in the spec's "default/on"
        // mode, where it reflects the content attribute and falls back to "on". Reading it
        // there reports a divergence the page never made, and the fallback is not equal to
        // the empty default, so every such control would carry an invented value.
        assert!(
            script.contains(
                "'textarea,input:not([type=checkbox]):not([type=radio]):not([type=file])'"
            )
        );
        // An entry is written only where the two disagree, so an untouched page records
        // nothing and the emitted spec does not grow for every control on it.
        assert!(script.contains("if (current === base(element)) continue;"));
        // A default that was turned off must survive as an explicit `null`; dropping the
        // entry would let the markup default win back and re-check a cleared box.
        assert!(script.contains("(current ? '' : null)"));
    }
}

/// The live value must never be smuggled in as a text child. An `<input>` is a void element
/// and cannot have children, so a text node there is not the control's value in any sense
/// the DOM or React recognises: it renders as stray text beside an empty control, while a
/// grep for the value still succeeds. Now that the value has a slot of its own, the
/// synthetic child is strictly wrong rather than merely odd.
#[test]
fn never_synthesises_a_control_value_as_a_text_child() {
    for script in [source(), crate::interaction_script::source()] {
        assert!(!script.contains("document.createTextNode(element.value)"));
    }
}
/// both must be kept in step. `direction` is inherited, so a page declares it once at the
/// root and every box it positions carries no declaration of its own — the authored style
/// map is right to leave those empty and cannot answer the question. The engine already
/// resolved it for the computed style being read on the same line, so the answer is taken
/// from there rather than re-derived by walking ancestors in a later stage.
#[test]
fn records_the_effective_direction_in_every_node_record() {
    for script in [source(), crate::interaction_script::source()] {
        assert!(script.contains("rtl: computedStyle.direction === 'rtl'"));
        assert!(!script.contains("element.dir"));
    }
}

/// The other inherited axis, recorded for the same reason from the same computed style.
/// `direction` chooses which end of the inline axis is its start; `writing-mode` chooses
/// which physical axis is inline at all, so a logical size resolves against it and lands
/// on the wrong dimension without it. Both producers are asserted together because a fix
/// applied to one leaves the arbiter behaving differently by state for no authored reason.
#[test]
fn records_the_effective_writing_mode_in_every_node_record() {
    for script in [source(), crate::interaction_script::source()] {
        assert!(script.contains("writing_mode: computedStyle.writingMode"));
    }
}
