use super::{responsive, state_styles};
use crate::model::{PageState, Specification};
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "state_band_tests.rs"]
mod band_tests;

pub fn append(
    specification: &Specification,
    state_classes: &[BTreeMap<String, String>],
    interaction_classes: &[Vec<BTreeMap<String, String>>],
    assets: &BTreeMap<String, String>,
    css: &mut String,
) {
    append_states(&specification.states, state_classes, assets, css);
    for (interaction, class_maps) in specification.interactions.iter().zip(interaction_classes) {
        append_states(&interaction.states, class_maps, assets, css);
    }
}

fn append_states(
    states: &[PageState],
    class_maps: &[BTreeMap<String, String>],
    assets: &BTreeMap<String, String>,
    css: &mut String,
) {
    let Some(base) = states.first() else {
        return;
    };
    let mut baseline = String::new();
    if let Some(classes) = class_maps.first() {
        state_styles::append(&base.state_styles, classes, assets, &mut baseline);
    }
    css.push_str(&baseline);
    let mut responsive: Vec<_> = states.iter().zip(class_maps).skip(1).collect();
    responsive.sort_by_key(|(state, _)| std::cmp::Reverse(state.viewport.width));
    for (index, (state, classes)) in responsive.iter().enumerate() {
        let mut rules = String::new();
        state_styles::append(&state.state_styles, classes, assets, &mut rules);
        // A band emitted directly after the base it repeats declares nothing new at any width.
        // Nothing is written between the two, so dropping it cannot change which rule wins.
        if rules == baseline {
            continue;
        }
        let wider = if index == 0 {
            base.viewport.width
        } else {
            responsive[index - 1].0.viewport.width
        };
        let smaller = responsive.get(index + 1).map(|next| next.0.viewport.width);
        let (minimum, maximum) =
            responsive::band(state.viewport.width, smaller, wider, responsive.len() == 1);
        css.push_str(&responsive::media_rule(minimum, maximum, &rules));
    }
}
