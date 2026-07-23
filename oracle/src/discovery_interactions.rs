use crate::model::{Obligation, ObligationStatus, Scenario, Step};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Deserialize)]
pub struct Control {
    pub(crate) anchor: String,
    pub(crate) stateful: bool,
    pub(crate) active: bool,
    pub(crate) popup: bool,
    pub(crate) group: String,
}

pub fn build(controls: &[Control]) -> (Vec<Scenario>, Vec<Obligation>) {
    let mut scenarios = Vec::new();
    let mut covered = BTreeMap::<&str, BTreeSet<String>>::new();
    let mut grouped = BTreeMap::<&str, Vec<&Control>>::new();
    for control in controls {
        if control.stateful {
            grouped.entry(&control.group).or_default().push(control);
        }
    }
    for controls in grouped.values().filter(|values| values.len() > 1) {
        let id = format!("state-sequence-{}", scenarios.len());
        let baseline = controls
            .iter()
            .find(|control| control.active)
            .copied()
            .unwrap_or(controls[0]);
        let mut steps = vec![Step::Reset];
        for control in controls {
            steps.push(activate(control));
            covered
                .entry(&control.anchor)
                .or_default()
                .insert(id.clone());
        }
        steps.push(activate(baseline));
        scenarios.push(Scenario { id, steps });
    }
    for control in controls {
        if covered.contains_key(control.anchor.as_str()) {
            continue;
        }
        let id = format!("interaction-{}", scenarios.len());
        let steps = if control.popup {
            vec![
                Step::Reset,
                activate(control),
                Step::Key {
                    key: "Escape".into(),
                },
                activate(control),
            ]
        } else if control.stateful {
            vec![Step::Reset, activate(control), activate(control)]
        } else {
            vec![
                Step::Reset,
                Step::Hover {
                    anchor: control.anchor.clone(),
                },
                activate(control),
            ]
        };
        covered
            .entry(&control.anchor)
            .or_default()
            .insert(id.clone());
        scenarios.push(Scenario { id, steps });
    }
    if !controls.is_empty() {
        scenarios.push(Scenario {
            id: "keyboard-navigation".into(),
            steps: std::iter::once(Step::Reset)
                .chain((0..controls.len().min(8)).map(|_| Step::Key { key: "Tab".into() }))
                .collect(),
        });
    }
    let obligations = controls
        .iter()
        .enumerate()
        .map(|(index, control)| Obligation {
            id: format!("control:{index}"),
            kind: "trusted-input".into(),
            status: ObligationStatus::Qualified,
            scenarios: covered
                .get(control.anchor.as_str())
                .into_iter()
                .flat_map(|values| values.iter().cloned())
                .collect(),
        })
        .collect();
    (scenarios, obligations)
}

fn activate(control: &Control) -> Step {
    Step::Activate {
        anchor: control.anchor.clone(),
    }
}
