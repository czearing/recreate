use super::authored_names;
use crate::model::{Animation, Node, PageState, Rect, Specification, Styles, Viewport};

const DEFINITION: &str = "@keyframes spin{from{rotate:0deg;}to{rotate:360deg;}}";
const PATH: &str = "html>body:nth-of-type(1)>div:nth-of-type(1)";

/// The set must describe the stylesheet this pipeline published, not the rules it was built
/// from, because those two answers differ wherever emission drops something.
///
/// `css_base::build` emits the authored rules only for the base pass; an interaction state
/// is built with `include_interactions` false and re-uses the base stylesheet, so its own
/// output carries no `@keyframes` even though its captured `css_rules` still do. Answering
/// from the captured rules there reports the name as already defined, the sampler declines
/// to rebuild it, and the state's `animation-name` points at a block no stylesheet holds —
/// the one failure shape with no fallback left, since suppressing the rebuild is exactly
/// what the reported defect's duplicate was. Answering from the output rebuilds it.
#[test]
fn a_name_reported_defined_is_defined_by_the_stylesheet_that_was_emitted() {
    let output = crate::generate::css::build_scoped(
        &specification(),
        &Default::default(),
        "s",
        false,
        None,
        None,
        None,
    );
    assert!(
        !output.css.contains("@keyframes spin"),
        "this pass does not emit authored rules, so the fixture cannot prove anything: {}",
        output.css
    );
    assert!(
        !authored_names(&output.css).contains("spin"),
        "a name this stylesheet does not define was reported as defined: {}",
        output.css
    );
    assert!(
        output.css.contains("@keyframes recreate"),
        "nothing defines the animation: the sampler was told the name was already authored:\n{}",
        output.css
    );
}
fn specification() -> Specification {
    let path = PATH;
    let mut styles = Styles::new();
    styles.insert("display".into(), "block".into());
    styles.insert("animation-name".into(), "spin".into());
    Specification {
        schema_version: 1,
        requested_url: "https://example.com".into(),
        captured_url: "https://example.com".into(),
        states: vec![PageState {
            url: "https://example.com".into(),
            title: "Example".into(),
            viewport: Viewport {
                width: 800,
                height: 600,
                dpr: 1.0,
            },
            nodes: vec![
                node("html", None, Styles::new()),
                node(path, Some("html"), styles),
            ],
            animations: vec![Animation {
                target: path.into(),
                name: "spin".into(),
                keyframes: vec![
                    serde_json::json!({"offset":0.0,"rotate":"0deg"}),
                    serde_json::json!({"offset":1.0,"rotate":"360deg"}),
                ],
                timing: serde_json::json!({"duration":4000,"iterations":"infinite"}),
            }],
            css_rules: vec![DEFINITION.into()],
            ..Default::default()
        }],
        interactions: Vec::new(),
        transitions: Vec::new(),
    }
}

fn node(path: &str, parent: Option<&str>, styles: Styles) -> Node {
    Node {
        scrollbar_gutter: 0.0,
        path: path.into(),
        parent: parent.map(str::to_string),
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 80.0,
        },
        style: styles,
        before: None,
        after: None,
        writing_mode: Default::default(),
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        ..Default::default()
    }
}
