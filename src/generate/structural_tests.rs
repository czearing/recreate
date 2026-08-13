use super::*;
use crate::model::{Attributes, Interaction, Node, PageState, Rect, Specification, Viewport};

fn node(path: &str, parent: Option<&str>, tag: &str, text: &str) -> Node {
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
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
        ..Default::default()
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
        transitions: Vec::new(),
    };
    let directory = tempfile::tempdir().unwrap();
    write_project(&specification, directory.path(), &[])
        .await
        .unwrap();
    let source = directory.path().join("react/src");
    let app = super::tests::read_source_tree(&source);
    let interactions = app.clone();
    let css = super::tests::read_css_tree(&source);
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

/// The document roots carry no class, so the root rule is the only path their styles
/// have to the output. Naming a subset of properties there drops the rest silently: a
/// `background` reset on `body` disappeared while the `margin` beside it survived.
#[tokio::test]
async fn the_root_rule_carries_every_captured_root_declaration() {
    let mut page = state(1200, false, false);
    let body = page
        .nodes
        .iter_mut()
        .find(|node| node.tag == "body")
        .unwrap();
    body.style
        .insert("background-color".into(), "rgb(255, 255, 255)".into());
    body.style.insert("margin-top".into(), "0px".into());
    let specification = Specification {
        schema_version: 1,
        requested_url: page.url.clone(),
        captured_url: page.url.clone(),
        states: vec![page],
        interactions: Vec::new(),
        transitions: Vec::new(),
    };
    let directory = tempfile::tempdir().unwrap();
    write_project(&specification, directory.path(), &[])
        .await
        .unwrap();
    let css = std::fs::read_to_string(directory.path().join("react/src/styles.css")).unwrap();
    let rule = css
        .split("body {")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .unwrap_or_default()
        .to_string();
    assert!(
        rule.contains("background-color:rgb(255, 255, 255)"),
        "{css}"
    );
    assert!(rule.contains("margin-top:0px"), "{css}");
}

/// Nothing may be shipped into a generated project that the project never uses. A dead import
/// is not merely noise: it names a module the reader will go and study, and it keeps a whole
/// runtime file alive that no code path can reach. `moveCarousel` was such a case - imported
/// into every app and every view, defined once, and called nowhere, a vestigial third copy of
/// the carousel state rule that had lost its caller.
#[tokio::test]
async fn imports_nothing_a_generated_file_does_not_use() {
    let directory = tempfile::tempdir().unwrap();
    write_project(
        &crate::generate::project_test_support::specification(),
        directory.path(),
        &[],
    )
    .await
    .unwrap();
    let mut checked = 0;
    let mut pending = vec![directory.path().join("react/src")];
    while let Some(path) = pending.pop() {
        for entry in std::fs::read_dir(&path).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if !matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("jsx" | "mjs" | "js")
            ) {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for line in source.lines().filter(|line| line.starts_with("import ")) {
                let Some(names) = line
                    .split_once('{')
                    .and_then(|(_, rest)| rest.split_once('}'))
                else {
                    continue;
                };
                for name in names
                    .0
                    .split(',')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                {
                    let name = name.rsplit(" as ").next().unwrap();
                    assert!(
                        source.matches(name).count() > 1,
                        "{} imports {name} and never uses it",
                        path.display()
                    );
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 0, "no named imports were examined");
}
