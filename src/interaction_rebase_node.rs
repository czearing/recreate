//! Field-by-field rebase of one node: a value this interaction did not change is reported
//! with its baseline value, so a later diff against the baseline stays silent about it.
use crate::model::{Node, Pseudo, Styles};
use std::collections::BTreeSet;

pub(super) fn rebase_node(state: &mut Node, fresh: &Node, baseline: &Node) {
    if state.text == fresh.text {
        state.text.clone_from(&baseline.text);
    }
    if state.rect == fresh.rect {
        state.rect.clone_from(&baseline.rect);
    }
    rebase_map(
        &mut state.attributes,
        &fresh.attributes,
        &baseline.attributes,
    );
    rebase_map(&mut state.style, &fresh.style, &baseline.style);
    rebase_pseudo(&mut state.before, &fresh.before, &baseline.before);
    rebase_pseudo(&mut state.after, &fresh.after, &baseline.after);
}

pub(super) fn rebase_pseudo(
    state: &mut Option<Pseudo>,
    fresh: &Option<Pseudo>,
    baseline: &Option<Pseudo>,
) {
    let (Some(state), Some(fresh), Some(baseline)) = (state, fresh, baseline) else {
        return;
    };
    if state.content == fresh.content {
        state.content.clone_from(&baseline.content);
    }
    rebase_map(&mut state.style, &fresh.style, &baseline.style);
}

pub(super) fn rebase_map(state: &mut Styles, fresh: &Styles, baseline: &Styles) {
    let keys: BTreeSet<_> = state.keys().chain(fresh.keys()).cloned().collect();
    for key in keys {
        if state.get(&key) != fresh.get(&key) {
            continue;
        }
        match baseline.get(&key) {
            Some(value) => {
                state.insert(key, value.clone());
            }
            None => {
                state.remove(&key);
            }
        }
    }
}
