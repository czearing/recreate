use crate::model::{Node, Styles};

const PROPERTIES: &[&str] = &["-webkit-text-fill-color", "color", "fill", "stroke"];

pub fn normalize<'a>(
    styles: &mut Styles,
    node: &Node,
    parent: Option<&Node>,
    rules: impl Into<super::authored_css::Authored<'a>>,
) {
    normalize_indexed(
        styles,
        node,
        parent,
        &super::authored_css::Index::new(rules),
    );
}

pub fn normalize_indexed(
    styles: &mut Styles,
    node: &Node,
    parent: Option<&Node>,
    rules: &super::authored_css::Index<'_>,
) {
    let Some(parent) = parent else {
        return;
    };
    for property in PROPERTIES {
        if disabled_control(node) && matches!(*property, "color" | "-webkit-text-fill-color") {
            continue;
        }
        if let Some(value) = rules.inherited_value(node, property) {
            styles.insert((*property).into(), value);
        } else if styles.get(*property) == parent.style.get(*property)
            && !(matches!(
                node.tag.as_str(),
                "button" | "input" | "select" | "textarea"
            ) && matches!(*property, "color" | "-webkit-text-fill-color"))
        {
            styles.remove(*property);
        }
    }

    /// Whether this control is painted as disabled, so authored paint must not be restored
    /// over what it actually showed.
    ///
    /// The real disabled state is not always borne by the element that shows it — a
    /// `<fieldset disabled>` disables every descendant control except those in its first
    /// `<legend>` — so it is answered by the engine at capture and read here. Re-deriving
    /// it from the attribute map, or from the `disabled` DOM property that merely reflects
    /// that map, is blind to every ancestor-borne case.
    ///
    /// `aria-disabled` stays an attribute read. It annotates assistive technology, changes
    /// no rendering and does not propagate, so it is a different question with a different
    /// answer and must not be folded into the recorded boolean.
    fn disabled_control(node: &Node) -> bool {
        node.disabled
            || (matches!(
                node.tag.as_str(),
                "button" | "input" | "select" | "textarea"
            ) && node
                .attributes
                .get("aria-disabled")
                .is_some_and(|value| value == "true"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Rect, Styles};

    fn node(tag: &str, class: &str, styles: &[(&str, &str)]) -> Node {
        let mut node = Node {
            writing_mode: Default::default(),
            scrollbar_gutter: 0.0,
            blocking_overlay: false,
            disabled: false,
            rtl: false,
            path: tag.into(),
            parent: None,
            tag: tag.into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 16.0,
                height: 16.0,
            },
            style: Styles::from_iter(
                styles
                    .iter()
                    .map(|(name, value)| ((*name).into(), (*value).into())),
            ),
            ..Default::default()
        };
        node.attributes.insert("class".into(), class.into());
        node
    }

    #[test]
    fn removes_computed_inherited_paint() {
        let parent = node("span", "control", &[("color", "rgb(138, 136, 134)")]);
        let child = node("path", "", &[("color", "rgb(138, 136, 134)")]);
        let mut styles = child.style.clone();
        normalize(&mut styles, &child, Some(&parent), &[]);
        assert!(!styles.contains_key("color"));
    }

    #[test]
    fn preserves_authored_current_color() {
        let parent = node("span", "control", &[("fill", "rgb(0, 0, 0)")]);
        let child = node("svg", "icon", &[("fill", "rgb(138, 136, 134)")]);
        let mut styles = child.style.clone();
        normalize(
            &mut styles,
            &child,
            Some(&parent),
            &[".icon{fill:currentColor;}".into()],
        );
        assert_eq!(styles["fill"], "currentColor");
    }

    #[test]
    fn preserves_form_control_text_color() {
        let parent = node("div", "", &[("color", "rgb(36, 36, 36)")]);
        let child = node("button", "", &[("color", "rgb(36, 36, 36)")]);
        let mut styles = child.style.clone();
        normalize(&mut styles, &child, Some(&parent), &[]);
        assert_eq!(styles["color"], "rgb(36, 36, 36)");
    }

    #[test]
    fn preserves_computed_disabled_control_paint() {
        let parent = node("div", "", &[("color", "rgb(36, 36, 36)")]);
        let mut child = node(
            "button",
            "send",
            &[
                ("color", "rgb(176, 176, 176)"),
                ("-webkit-text-fill-color", "rgb(176, 176, 176)"),
            ],
        );
        child.attributes.insert("disabled".into(), String::new());
        child.disabled = true;
        let mut styles = child.style.clone();
        normalize(
            &mut styles,
            &child,
            Some(&parent),
            &[".send{color:white;-webkit-text-fill-color:white;}".into()],
        );
        assert_eq!(styles["color"], "rgb(176, 176, 176)");
        assert_eq!(styles["-webkit-text-fill-color"], "rgb(176, 176, 176)");
    }

    /// A `<fieldset disabled>` disables every descendant control, which the descendant's
    /// own attribute map cannot show. The control is really disabled — it matches
    /// `:disabled` and paints as disabled — so restoring the class-authored paint over
    /// what it actually showed paints an enabled-looking control.
    #[test]
    fn preserves_computed_paint_of_a_control_disabled_by_an_ancestor() {
        let parent = node("fieldset", "", &[("color", "rgb(36, 36, 36)")]);
        let mut child = node(
            "button",
            "send",
            &[
                ("color", "rgb(176, 176, 176)"),
                ("-webkit-text-fill-color", "rgb(176, 176, 176)"),
            ],
        );
        child.disabled = true;
        assert!(
            !child.attributes.contains_key("disabled"),
            "the fixture must carry the state only where the page did"
        );
        let mut styles = child.style.clone();
        normalize(
            &mut styles,
            &child,
            Some(&parent),
            &[".send{color:white;-webkit-text-fill-color:white;}".into()],
        );
        assert_eq!(styles["color"], "rgb(176, 176, 176)");
        assert_eq!(styles["-webkit-text-fill-color"], "rgb(176, 176, 176)");
    }

    /// The scope control. A control inside the fieldset's first `<legend>` stays enabled,
    /// so it does not match `:disabled` and its authored paint is the correct paint. A
    /// repair that walks for a disabled ancestor instead of asking the engine greys it.
    #[test]
    fn restores_authored_paint_for_an_enabled_control_under_a_disabled_ancestor() {
        let parent = node("legend", "", &[("color", "rgb(36, 36, 36)")]);
        let child = node("button", "send", &[("color", "rgb(255, 255, 255)")]);
        let mut styles = child.style.clone();
        normalize(
            &mut styles,
            &child,
            Some(&parent),
            &[".send{color:white;}".into()],
        );
        assert_eq!(styles["color"], "white");
    }

    /// The propagation control. `aria-disabled` annotates assistive technology; it changes
    /// no rendering and has no descendant semantics, so a button under an
    /// `aria-disabled` fieldset is painted exactly as authored.
    #[test]
    fn restores_authored_paint_under_an_aria_disabled_ancestor() {
        let parent = node("fieldset", "", &[("color", "rgb(36, 36, 36)")]);
        let mut parent = parent;
        parent
            .attributes
            .insert("aria-disabled".into(), "true".into());
        let child = node("button", "send", &[("color", "rgb(255, 255, 255)")]);
        let mut styles = child.style.clone();
        normalize(
            &mut styles,
            &child,
            Some(&parent),
            &[".send{color:white;}".into()],
        );
        assert_eq!(styles["color"], "white");
    }

    /// `aria-disabled` on the control itself keeps its existing meaning. It is not folded
    /// into the engine-answered boolean, because doing so would grey the propagation case.
    #[test]
    fn preserves_computed_paint_for_a_self_aria_disabled_control() {
        let parent = node("div", "", &[("color", "rgb(36, 36, 36)")]);
        let mut child = node("button", "send", &[("color", "rgb(176, 176, 176)")]);
        child
            .attributes
            .insert("aria-disabled".into(), "true".into());
        let mut styles = child.style.clone();
        normalize(
            &mut styles,
            &child,
            Some(&parent),
            &[".send{color:white;}".into()],
        );
        assert_eq!(styles["color"], "rgb(176, 176, 176)");
    }
}
