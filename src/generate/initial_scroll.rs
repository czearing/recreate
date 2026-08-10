use super::scroll_state;
use crate::model::{PageState, Specification};

pub fn targets(specification: &Specification) -> String {
    serde_json::to_string(
        &specification
            .states
            .iter()
            .map(snapshot)
            .collect::<Vec<_>>(),
    )
    .expect("initial scroll targets should serialize")
}

fn snapshot(state: &PageState) -> Vec<(&str, i64, i64)> {
    scroll_state::resting(state)
        .into_iter()
        .map(|scrolled| (scrolled.path, scrolled.left, scrolled.top))
        .collect()
}
