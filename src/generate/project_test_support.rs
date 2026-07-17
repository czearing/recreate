use crate::model::{
    Animation, Attributes, Interaction, Node, PageState, Pseudo, Rect, Specification, Styles,
    Viewport,
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
    let card_width = match width {
        0..=320 => "70px",
        321..=390 => "80px",
        391..=768 => "90px",
        769..=1440 => "95px",
        _ => "100px",
    };
    for index in [3, 5] {
        nodes[index].style.insert("width".into(), card_width.into());
    }
    nodes[3].before = match width {
        0..=320 => None,
        321..=390 => Some(pseudo("\"mobile\"", "blue")),
        _ => Some(pseudo("\"wide\"", "red")),
    };
    PageState {
        url: "https://example.com".into(),
        title: "Example".into(),
        viewport: Viewport {
            width,
            height: 800,
            dpr: 1.0,
        },
        nodes,
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: vec![Animation {
            target: "html>body:nth-of-type(1)>div:nth-of-type(1)".into(),
            keyframes: vec![
                json!({"offset":0,"opacity":"0"}),
                json!({"offset":1,"opacity":"1"}),
            ],
            timing: json!({"duration":200}),
        }],
        state_styles: Vec::new(),
        css_rules: Vec::new(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}

fn pseudo(content: &str, color: &str) -> Pseudo {
    Pseudo {
        content: content.into(),
        style: Styles::from([("color".into(), color.into())]),
    }
}

pub fn specification() -> Specification {
    let mut states = [1920, 1440, 768, 390, 320].map(state).to_vec();
    for state in &mut states {
        states_semantics(state, "false");
    }
    let mut interaction_states = states.clone();
    for state in &mut interaction_states {
        states_semantics(state, "true");
        state.nodes[5]
            .attributes
            .insert("role".into(), "dialog".into());
        state.nodes[5]
            .attributes
            .insert("aria-modal".into(), "true".into());
    }
    Specification {
        schema_version: 1,
        requested_url: states[0].url.clone(),
        captured_url: states[0].url.clone(),
        states: states.clone(),
        interactions: vec![Interaction {
            trigger_path: "html>body:nth-of-type(1)>div:nth-of-type(1)>div:nth-of-type(1)".into(),
            trigger_tag: "div".into(),
            trigger_label: "Card".into(),
            focused_path: Some(
                "html>body:nth-of-type(1)>div:nth-of-type(1)>div:nth-of-type(2)".into(),
            ),
            states: interaction_states,
        }],
    }
}

fn states_semantics(state: &mut PageState, expanded: &str) {
    state.nodes[3].attributes.extend([
        ("role".into(), "button".into()),
        ("tabindex".into(), "0".into()),
        ("aria-haspopup".into(), "dialog".into()),
        ("aria-expanded".into(), expanded.into()),
    ]);
}
