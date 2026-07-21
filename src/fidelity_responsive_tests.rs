use crate::fidelity_responsive::{ResponsiveFrame, ResponsiveNode, compare};
use std::collections::BTreeMap;

fn node(depth: usize, height: f64) -> ResponsiveNode {
    ResponsiveNode {
        depth,
        tag: "div".into(),
        rect: [0.0, 0.0, 100.0, height],
        style: BTreeMap::from([("height".into(), format!("{height}px"))]),
    }
}

fn frame(ancestors: Vec<ResponsiveNode>, document_height: f64) -> ResponsiveFrame {
    ResponsiveFrame {
        width: 480,
        identity_stable: true,
        ancestors,
        flow: Vec::new(),
        text: Vec::new(),
        document: [480.0, document_height],
    }
}

#[test]
fn ignores_geometry_outside_component_context() {
    let source = frame(
        vec![node(0, 180.0), node(1, 480.0), node(2, 9_380.0)],
        10_000.0,
    );
    let candidate = frame(
        vec![node(0, 180.0), node(1, 480.0), node(2, 4_780.0)],
        5_400.0,
    );

    assert!(compare(&[source], &[candidate]).is_empty());
}

#[test]
fn rejects_root_and_parent_geometry_mismatches() {
    let source = frame(vec![node(0, 180.0), node(1, 480.0)], 1_000.0);
    let candidate = frame(vec![node(0, 160.0), node(1, 460.0)], 1_000.0);

    assert!(!compare(&[source], &[candidate]).is_empty());
}
