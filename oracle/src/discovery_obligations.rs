use crate::model::{Obligation, ObligationStatus};
use std::collections::BTreeSet;

pub fn append(value: &serde_json::Value, controls_empty: bool, obligations: &mut Vec<Obligation>) {
    for kind in ["timer", "animation", "media", "container", "observer"] {
        if value["registrations"][kind] == true {
            obligations.push(Obligation {
                id: format!("registration:{kind}"),
                kind: kind.into(),
                status: ObligationStatus::Qualified,
                scenarios: vec![scenario_for(kind).into()],
            });
        }
    }
    let runtime_kinds = value["runtime"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|registration| registration["kind"].as_str().unwrap_or("unknown"))
        .collect::<BTreeSet<_>>();
    for kind in runtime_kinds {
        let scenario = runtime_scenario(kind, controls_empty);
        obligations.push(Obligation {
            id: format!("runtime:{kind}"),
            kind: kind.into(),
            status: if scenario != "uncovered" {
                ObligationStatus::Qualified
            } else {
                ObligationStatus::Uncovered
            },
            scenarios: if scenario != "uncovered" {
                vec![scenario.into()]
            } else {
                Vec::new()
            },
        });
    }
    if value["motionUnbounded"] == true {
        obligations.push(Obligation {
            id: "motion:unbounded".into(),
            kind: "motion".into(),
            status: ObligationStatus::Uncovered,
            scenarios: Vec::new(),
        });
    }
}

fn scenario_for(kind: &str) -> &str {
    match kind {
        "timer" => "async-settled",
        "animation" => "motion-frames",
        "media" | "container" | "observer" => "responsive-ascending",
        _ => "uncovered",
    }
}

fn runtime_scenario(kind: &str, controls_empty: bool) -> &str {
    match kind {
        "timeout" | "interval" | "microtask" | "fetch" => "async-settled",
        "raf" | "waapi" => "motion-frames",
        "media" => "responsive-discovered",
        "listener" if !controls_empty => "keyboard-navigation",
        "shadow" => "responsive-ascending",
        _ => "uncovered",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_registrations_are_deduplicated_by_kind() {
        let value = serde_json::json!({
            "registrations": {},
            "runtime": [
                {"kind": "timeout"},
                {"kind": "listener"},
                {"kind": "timeout"}
            ]
        });
        let mut obligations = Vec::new();
        super::append(&value, false, &mut obligations);
        assert_eq!(obligations.len(), 2);
        assert_eq!(obligations[0].id, "runtime:listener");
        assert_eq!(obligations[1].id, "runtime:timeout");
    }
}
