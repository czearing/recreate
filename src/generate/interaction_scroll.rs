use super::scroll_state;
use crate::model::{PageState, Specification};

pub fn targets(specification: &Specification) -> String {
    let interactions = specification
        .interactions
        .iter()
        .map(|interaction| {
            let values = interaction
                .states
                .iter()
                .map(|state| {
                    specification
                        .states
                        .iter()
                        .find(|baseline| baseline.viewport.width == state.viewport.width)
                        .map(|baseline| scroll_snapshot(baseline, state))
                        .unwrap_or_else(|| "null".into())
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("[{values}]")
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[null,{interactions}]")
}

pub fn moves_horizontally(
    interaction: &crate::model::Interaction,
    baselines: &[PageState],
) -> bool {
    interaction.states.iter().any(|state| {
        baselines
            .iter()
            .find(|baseline| baseline.viewport.width == state.viewport.width)
            .is_some_and(|baseline| scroll_state::shifted_horizontally(baseline, state))
    })
}

/// Serializes the offsets the action left the page holding, splitting the one element the
/// runtime reaches through the global `scrollTo` from the ones it reaches by `querySelector`.
fn scroll_snapshot(baseline: &PageState, state: &PageState) -> String {
    let scrolled = scroll_state::moved(baseline, state);
    if scrolled.is_empty() {
        return "null".into();
    }
    let (document, elements): (Vec<_>, Vec<_>) = scrolled
        .into_iter()
        .partition(|scrolled| scroll_state::scrolls_document(state, scrolled.path()));
    let window = document
        .first()
        .map_or((0, 0), |scrolled| (scrolled.left(), scrolled.top()));
    let elements = elements
        .into_iter()
        .map(|scrolled| {
            format!(
                "[{}, {},{}]",
                serde_json::to_string(scrolled.path()).unwrap(),
                scrolled.left(),
                scrolled.top()
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{window:[{},{}],elements:[{elements}]}}",
        window.0, window.1
    )
}
