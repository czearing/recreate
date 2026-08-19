use super::responsive;
use crate::model::{Node, PageState, Rect, Specification, Styles, Viewport};
use std::collections::BTreeMap;

fn viewport(width: u32) -> Viewport {
    Viewport {
        width,
        height: 900,
        dpr: 1.0,
    }
}

fn item(path: &str, width: f64) -> Node {
    Node {
        path: path.into(),
        parent: Some("html>body".into()),
        tag: "div".into(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width,
            height: 20.0,
        },
        style: Styles::from([("margin-left".into(), format!("{width}px"))]),
        ..Default::default()
    }
}

fn state(width: u32, margins: [f64; 3]) -> PageState {
    PageState {
        viewport: viewport(width),
        nodes: vec![
            item("html>body>a", margins[0]),
            item("html>body>b", margins[1]),
            item("html>body>c", margins[2]),
        ],
        ..Default::default()
    }
}

fn band_css(classes: [(&str, &str); 3]) -> String {
    let specification = Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![state(1440, [10.0, 10.0, 10.0]), state(768, [0.0, 0.0, 5.0])],
        interactions: Vec::new(),
        transitions: Vec::new(),
    };
    let classes: BTreeMap<String, String> = classes
        .iter()
        .map(|(path, class)| ((*path).to_string(), (*class).to_string()))
        .collect();
    let mut css = String::new();
    responsive::append_filtered(
        &specification,
        &Default::default(),
        &classes,
        &mut css,
        None,
        &Default::default(),
    );
    css
}

/// Three elements narrow at the same viewport; two of them land on the same declarations. A
/// band is a set of declarations and the elements that need them, so the pair is one rule with
/// a selector list. Emitting the body once per class states the same thing as many times as
/// there are elements, which is how a stylesheet grows past the page it recreates.
#[test]
fn states_one_band_body_once_for_every_class_that_needs_it() {
    let css = band_css([
        ("html>body>a", "ra"),
        ("html>body>b", "rb"),
        ("html>body>c", "rc"),
    ]);
    assert!(css.contains(".ra,.rb{margin-left:0px;}"), "{css}");
    assert_eq!(css.matches("margin-left:0px;").count(), 1, "{css}");
    assert!(css.contains(".rc{margin-left:5px;}"), "{css}");
}

/// The grouping is by declarations, not by class, so an element whose class is shared with
/// another element is still named once. Listing a class twice in one selector is invalid to
/// read and says nothing the single mention does not.
#[test]
fn never_names_one_class_twice_in_a_grouped_band_selector() {
    let css = band_css([
        ("html>body>a", "shared"),
        ("html>body>b", "shared"),
        ("html>body>c", "rc"),
    ]);
    assert!(css.contains(".shared{margin-left:0px;}"), "{css}");
    assert_eq!(css.matches(".shared").count(), 1, "{css}");
}

/// The same rule one layer up. Two elements that rest identically are two classes exactly when
/// they answer a state differently, and their resting declarations are then written twice —
/// once per class — unless the base layer groups by body as the bands do.
#[test]
fn states_one_resting_body_once_for_every_class_that_needs_it() {
    let nodes = vec![
        crate::generate::css_pseudo_identity_tests::span(1),
        crate::generate::css_pseudo_identity_tests::span(2),
    ];
    let paths: Vec<String> = nodes.iter().map(|node| node.path.clone()).collect();
    let specification = Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![PageState {
            viewport: viewport(1440),
            nodes,
            state_styles: vec![crate::model::StateStyle {
                target: paths[0].clone(),
                scope: None,
                relation: Default::default(),
                pseudo: Some(":hover".into()),
                target_pseudo: None,
                media: None,
                declarations: "color: red;".into(),
            }],
            ..Default::default()
        }],
        interactions: Vec::new(),
        transitions: Vec::new(),
    };
    let output = crate::generate::css_base::build(crate::generate::css_base::Request {
        specification: &specification,
        assets: &Default::default(),
        prefix: "r",
        include_interactions: true,
        reuse: None,
        cache: None,
        path_override: None,
        timing: &|_: &str| {},
    });
    let one = &output.classes[&paths[0]];
    let two = &output.classes[&paths[1]];
    assert_ne!(one, two, "the hovered element kept the other's identity");
    assert_eq!(
        output.css.matches("display:inline-block").count(),
        1,
        "the shared resting body was written once per class\n{}",
        output.css
    );
    assert!(
        output.css.contains(&format!(".{one},.{two}{{"))
            || output.css.contains(&format!(".{two},.{one}{{")),
        "{}",
        output.css
    );
}
