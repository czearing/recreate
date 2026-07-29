use crate::model::{Scenario, Step};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Control {
    pub(crate) anchor: String,
}

pub fn keyboard(controls: &[Control]) -> Option<Scenario> {
    (!controls.is_empty()).then(|| Scenario {
        id: "keyboard-navigation".into(),
        steps: std::iter::once(Step::Reset)
            .chain((0..controls.len()).map(|_| Step::Key { key: "Tab".into() }))
            .collect(),
    })
}
