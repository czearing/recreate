use crate::{
    checkpoint::DOMAINS,
    compare_difference,
    model::{Artifact, Checkpoint},
    report::{Difference, Report},
};
use serde_json::Value;
use std::{collections::BTreeMap, time::Duration};

pub fn artifacts(expected: &Artifact, actual: &[Checkpoint], elapsed: Duration) -> Report {
    artifacts_with_mode(expected, actual, elapsed, true)
}

pub fn reference_artifacts(
    expected: &Artifact,
    actual: &[Checkpoint],
    elapsed: Duration,
) -> Report {
    artifacts_with_mode(expected, actual, elapsed, false)
}

fn artifacts_with_mode(
    expected: &Artifact,
    actual: &[Checkpoint],
    elapsed: Duration,
    fast: bool,
) -> Report {
    let keyed = actual
        .iter()
        .map(|item| ((item.scenario.as_str(), item.step), item))
        .collect::<BTreeMap<_, _>>();
    let mut differences = Vec::new();
    let mut matched = 0;
    let mut domain_matches = BTreeMap::<String, usize>::new();
    let total = expected.checkpoints.len() * DOMAINS.len();
    for checkpoint in &expected.checkpoints {
        let Some(candidate) = keyed.get(&(checkpoint.scenario.as_str(), checkpoint.step)) else {
            differences.push(missing(checkpoint));
            continue;
        };
        for domain in DOMAINS {
            let expected_domain = checkpoint.domains.get(*domain);
            let actual_domain = candidate.domains.get(*domain);
            match (expected_domain, actual_domain) {
                (Some(left), Some(right))
                    if (fast && left.digest == right.digest)
                        || (!fast && left.value == right.value) =>
                {
                    matched += 1;
                    *domain_matches.entry((*domain).into()).or_default() += 1;
                }
                (Some(left), Some(right)) => {
                    let (path, expected, actual) =
                        compare_difference::between(&left.value, &right.value);
                    differences.push(Difference {
                        scenario: checkpoint.scenario.clone(),
                        step: checkpoint.step,
                        viewport_width: checkpoint.viewport.width,
                        domain: (*domain).into(),
                        path,
                        expected,
                        actual,
                    });
                }
                _ => differences.push(missing_domain(checkpoint, domain)),
            }
        }
    }
    let blockers = coverage_blockers(expected, actual);
    let certified = differences.is_empty() && blockers.is_empty() && matched == total;
    let first_difference = differences.first().cloned();
    let domain_scores_ppm = DOMAINS
        .iter()
        .map(|domain| {
            let count = domain_matches.get(*domain).copied().unwrap_or_default();
            let total = expected.checkpoints.len();
            (
                (*domain).into(),
                if total == 0 {
                    0
                } else {
                    ((count as u64 * 1_000_000) / total as u64) as u32
                },
            )
        })
        .collect();
    Report {
        status: if certified { "PASS" } else { "FAIL" }.into(),
        certified,
        accuracy_ppm: if total == 0 {
            0
        } else {
            ((matched as u64 * 1_000_000) / total as u64) as u32
        },
        qualified_coverage_ppm: if blockers.is_empty() { 1_000_000 } else { 0 },
        environment_digest: crate::digest::json(&expected.environment).unwrap_or_default(),
        source_artifact_digest: expected.payload_digest.clone(),
        domain_scores_ppm,
        matched_domains: matched,
        total_domains: total,
        first_difference,
        differences,
        blockers,
        elapsed_ms: elapsed.as_millis(),
    }
}

fn coverage_blockers(expected: &Artifact, actual: &[Checkpoint]) -> Vec<String> {
    let mut blockers = expected.coverage.incomplete.clone();
    if actual.len() != expected.checkpoints.len() {
        blockers.push(format!(
            "checkpoint coverage expected={} actual={}",
            expected.checkpoints.len(),
            actual.len()
        ));
    }
    blockers
}

fn missing(checkpoint: &Checkpoint) -> Difference {
    Difference {
        scenario: checkpoint.scenario.clone(),
        step: checkpoint.step,
        viewport_width: checkpoint.viewport.width,
        domain: "checkpoint".into(),
        path: "$".into(),
        expected: serde_json::json!("present"),
        actual: Value::Null,
    }
}

fn missing_domain(checkpoint: &Checkpoint, domain: &str) -> Difference {
    Difference {
        domain: domain.into(),
        ..missing(checkpoint)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn digest_equality_matches_slow_reference(left in any::<Vec<u8>>(), right in any::<Vec<u8>>()) {
            let left_value = serde_json::json!(left);
            let right_value = serde_json::json!(right);
            let optimized = crate::digest::json(&left_value).unwrap()
                == crate::digest::json(&right_value).unwrap();
            let reference = left_value == right_value;
            prop_assert_eq!(optimized, reference);
        }
    }
}
