use super::*;
use crate::model::{Attributes, Interaction, Node, PageState, Rect, Specification, Viewport};

fn node(path: &str, parent: Option<&str>, tag: &str, text: &str) -> Node {
    Node {
        path: path.into(),
        parent: parent.map(str::to_string),
        tag: tag.into(),
        text: text.into(),
        attributes: Attributes::new(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
        },
        style: [("display".into(), "block".into())].into(),
        before: None,
        after: None,
    }
}

fn state(width: u32, mobile: bool, expanded: bool) -> PageState {
    let body = "html>body:nth-of-type(1)";
    let root = format!("{body}>div:nth-of-type(1)");
    let main = format!("{root}>main:nth-of-type(1)");
    let trigger = format!("{main}>button:nth-of-type(1)");
    let branch = if mobile {
        format!("{main}>section:nth-of-type(1)")
    } else {
        format!("{main}>nav:nth-of-type(1)")
    };
    let mut nodes = vec![
        node("html", None, "html", ""),
        node(body, Some("html"), "body", ""),
        node(&root, Some(body), "div", ""),
        node(&main, Some(&root), "main", ""),
        node(&trigger, Some(&main), "button", ""),
        node(
            &format!("{trigger}>#text(1)"),
            Some(&trigger),
            "#text",
            if mobile {
                "Mobile menu"
            } else {
                "Desktop menu"
            },
        ),
        node(
            &branch,
            Some(&main),
            if mobile { "section" } else { "nav" },
            "",
        ),
    ];
    nodes[2].attributes.insert("id".into(), "root".into());
    nodes[4]
        .attributes
        .insert("aria-expanded".into(), expanded.to_string());
    if expanded {
        let dialog = format!("{main}>div:nth-of-type(1)");
        nodes.push(node(&dialog, Some(&main), "div", ""));
        nodes.last_mut().unwrap().attributes.extend([
            ("role".into(), "dialog".into()),
            ("aria-modal".into(), "true".into()),
        ]);
    }
    PageState {
        url: "https://example.test".into(),
        title: "Structural".into(),
        viewport: Viewport {
            width,
            height: 800,
            dpr: 1.0,
        },
        dom: Default::default(),
        capture_blockers: Vec::new(),
        nodes,
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}

#[tokio::test]
async fn writes_exact_viewport_and_interaction_structures() {
    let states = vec![state(1200, false, false), state(390, true, false)];
    let interaction_states = vec![state(1200, false, true), state(390, true, true)];
    let trigger = states[0].nodes[4].path.clone();
    let focused = interaction_states[0].nodes.last().unwrap().path.clone();
    let specification = Specification {
        schema_version: 1,
        requested_url: states[0].url.clone(),
        captured_url: states[0].url.clone(),
        states,
        interactions: vec![Interaction {
            trigger_path: trigger,
            trigger_tag: "button".into(),
            trigger_label: "Desktop menu".into(),
            trigger_occurrence: None,
            focused_path: Some(focused),
            states: interaction_states,
        }],
    };
    let directory = tempfile::tempdir().unwrap();
    write_project(&specification, directory.path(), &[])
        .await
        .unwrap();
    let source = directory.path().join("react/src");
    let app = std::fs::read_to_string(source.join("App.jsx")).unwrap();
    let interactions = std::fs::read_to_string(source.join("states.jsx")).unwrap();
    let css = std::fs::read_to_string(source.join("styles.css")).unwrap();
    assert!(app.contains("function Baseline0"));
    assert!(app.contains("function Baseline1"));
    assert!(app.contains("const viewportWidths=[1200,390]"));
    assert!(app.contains("if(width>widths[index+1])return index"));
    assert!(app.contains("matchMedia(`(max-width:${width}px)`)"));
    assert!(app.contains("Desktop menu") && app.contains("Mobile menu"));
    assert!(interactions.contains("Interaction1View0"));
    assert!(interactions.contains("Interaction1View1"));
    assert!(interactions.contains("role={\"dialog\"}"));
    assert!(interactions.contains("focus({preventScroll:true})"));
    assert!(
        css.contains(".s"),
        "mobile-only structure needs generated CSS"
    );
}

#[test]
fn jsx_viewport_selector_matches_responsive_bands() {
    let selector = super::jsx_variants::selector();
    assert!(selector.contains("width>widths[index+1]"));
    assert!(!selector.contains("width>=widths[index]"));
}
