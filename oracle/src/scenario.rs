use crate::model::{Scenario, Step};
use std::collections::BTreeSet;

pub fn responsive(widths: &[u32], height: u32) -> anyhow::Result<Vec<Scenario>> {
    let widths = widths.iter().copied().collect::<BTreeSet<_>>();
    anyhow::ensure!(!widths.is_empty(), "at least one width is required");
    anyhow::ensure!(
        widths.iter().all(|width| *width > 0 && *width <= 16_384),
        "widths must be within 1..=16384"
    );
    let ascending = widths.iter().copied().collect::<Vec<_>>();
    let descending = ascending.iter().rev().copied().collect::<Vec<_>>();
    Ok(vec![
        Scenario {
            id: "responsive-ascending".into(),
            steps: vec![Step::ResizePath {
                widths: ascending,
                height,
            }],
        },
        Scenario {
            id: "responsive-descending".into(),
            steps: vec![Step::ResizePath {
                widths: descending,
                height,
            }],
        },
        Scenario {
            id: "async-settled".into(),
            steps: vec![Step::AdvanceTime {
                milliseconds: 100_000,
            }],
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_cover_both_directions() {
        let scenarios = responsive(&[768, 320, 768], 800).unwrap();
        assert_eq!(scenarios.len(), 3);
        assert_eq!(
            scenarios[0].steps,
            vec![Step::ResizePath {
                widths: vec![320, 768],
                height: 800
            }]
        );
    }
}
