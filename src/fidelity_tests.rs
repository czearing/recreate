use super::*;

fn frame(path: &str, properties: &[&str]) -> Frame {
    Frame {
        elapsed_ms: 0,
        snapshot: Snapshot {
            nodes: vec![NodeSnapshot {
                path: path.into(),
                tag: "span".into(),
                class_name: "target".into(),
                rect: [0.0, 0.0, 20.0, 20.0],
                style: BTreeMap::from([
                    ("opacity".into(), "1".into()),
                    ("color".into(), "rgb(0, 0, 0)".into()),
                    ("backgroundColor".into(), "rgba(0, 0, 0, 0)".into()),
                    ("boxShadow".into(), "none".into()),
                ]),
            }],
            animations: vec![AnimationSnapshot {
                target: path.into(),
                pseudo: None,
                current_time: 0.0,
                duration: 180.0,
                delay: 0.0,
                easing: "ease".into(),
                properties: properties.iter().map(|value| (*value).into()).collect(),
            }],
            document: [100.0, 100.0],
            root_hovered: true,
            hit_path: Some(path.into()),
            visibility: "visible".into(),
        },
    }
}

#[test]
fn reports_missing_hover_property() {
    let source = Trace {
        label: "card".into(),
        hover: vec![frame(
            "span:nth-of-type(1)",
            &["opacity", "backgroundColor"],
        )],
        leave: Vec::new(),
    };
    let candidate = Trace {
        label: "card".into(),
        hover: vec![frame("span:nth-of-type(1)", &["opacity"])],
        leave: Vec::new(),
    };
    assert!(
        compare(&source, &candidate)
            .iter()
            .any(|detail| detail.contains("properties"))
    );
}

#[test]
fn accepts_matching_motion_and_geometry() {
    let trace = Trace {
        label: "card".into(),
        hover: vec![frame("span:nth-of-type(1)", &["opacity", "transform"])],
        leave: Vec::new(),
    };
    assert!(compare(&trace, &trace).is_empty());
}
