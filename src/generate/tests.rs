use super::*;
use crate::model::{
    Animation, Attributes, Interaction, Node, PageState, Rect, Specification, Styles, Viewport,
};
use serde_json::json;

fn node(path: &str, parent: Option<&str>, text: &str, test_id: Option<&str>) -> Node {
    let mut attributes = Attributes::new();
    if let Some(test_id) = test_id {
        attributes.insert("data-testid".into(), test_id.into());
    }
    let mut style = Styles::new();
    style.insert("display".into(), "block".into());
    style.insert("width".into(), "100px".into());
    Node {
        path: path.into(),
        parent: parent.map(str::to_string),
        tag: if path.contains("#text") {
            "#text"
        } else {
            "div"
        }
        .into(),
        text: text.into(),
        attributes,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
        },
        style,
        before: None,
        after: None,
    }
}

fn state(width: u32) -> PageState {
    let mut nodes = vec![
        node("html", None, "", None),
        node("html>body:nth-of-type(1)", Some("html"), "", None),
        node(
            "html>body:nth-of-type(1)>div:nth-of-type(1)",
            Some("html>body:nth-of-type(1)"),
            "",
            None,
        ),
    ];
    for index in 1..=2 {
        let root = format!("html>body:nth-of-type(1)>div:nth-of-type(1)>div:nth-of-type({index})");
        nodes.push(node(
            &root,
            Some("html>body:nth-of-type(1)>div:nth-of-type(1)"),
            "",
            Some("result-card"),
        ));
        nodes.push(node(
            &format!("{root}>#text(1)"),
            Some(&root),
            &format!("Card {index}"),
            None,
        ));
    }
    if width < 500 {
        nodes[3].style.insert("width".into(), "80px".into());
        nodes[5].style.insert("width".into(), "80px".into());
    }
    PageState {
        url: "https://example.com".into(),
        title: "Example".into(),
        viewport: Viewport {
            width,
            height: 800,
            dpr: 1.0,
        },
        nodes,
        animations: vec![Animation {
            target: "html>body:nth-of-type(1)>div:nth-of-type(1)".into(),
            keyframes: vec![
                json!({"offset":0,"opacity":"0"}),
                json!({"offset":1,"opacity":"1"}),
            ],
            timing: json!({"duration":200}),
        }],
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}

fn specification() -> Specification {
    let desktop = state(1200);
    let mobile = state(390);
    Specification {
        schema_version: 1,
        requested_url: desktop.url.clone(),
        captured_url: desktop.url.clone(),
        states: vec![desktop.clone(), mobile.clone()],
        interactions: vec![Interaction {
            trigger_path: "html>body:nth-of-type(1)>div:nth-of-type(1)>div:nth-of-type(1)".into(),
            trigger_tag: "div".into(),
            trigger_label: "Card".into(),
            states: vec![desktop, mobile],
        }],
    }
}

#[tokio::test]
async fn writes_semantic_component_project() {
    let directory = tempfile::tempdir().unwrap();
    let specification = specification();
    let components = super::tree::components(&specification, &Default::default());
    assert!(
        components
            .items
            .iter()
            .any(|item| item.name == "ResultCard"),
        "{:?}",
        components
            .items
            .iter()
            .map(|item| &item.name)
            .collect::<Vec<_>>()
    );
    write_project(&specification, directory.path(), &[])
        .await
        .unwrap();
    let root = directory.path().join("react");
    let index = std::fs::read_to_string(root.join("src/components/index.js")).unwrap();
    assert!(index.contains("ResultCard"), "{index}");
    assert!(root.join("src/states.jsx").exists());
    let app = std::fs::read_to_string(root.join("src/App.jsx")).unwrap();
    assert!(app.contains("Interaction1"));
    let css = std::fs::read_to_string(root.join("src/styles.css")).unwrap();
    assert!(css.contains("@media(max-width:390px)"));
    assert!(css.contains("@keyframes"));
}
