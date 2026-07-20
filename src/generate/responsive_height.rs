use crate::model::{Node, Styles, Viewport};

pub fn normalize(styles: &mut Styles, node: &Node, viewport: &Viewport) {
    let viewport_height = f64::from(viewport.height);
    let bottom = node.rect.y + node.rect.height;
    if node.rect.height < viewport_height * 0.7 || (bottom - viewport_height).abs() > 1.0 {
        return;
    }
    let padding = if node
        .style
        .get("box-sizing")
        .is_some_and(|value| value == "content-box")
    {
        vertical_padding(&node.style)
    } else {
        0.0
    };
    let offset = node.rect.y + padding;
    let height = if offset.abs() <= 1.0 {
        "100vh".into()
    } else if offset > 0.0 {
        format!("calc(100vh - {offset}px)")
    } else {
        return;
    };
    styles.insert("height".into(), height);
}

fn vertical_padding(styles: &Styles) -> f64 {
    ["padding-top", "padding-bottom"]
        .into_iter()
        .filter_map(|key| styles.get(key)?.strip_suffix("px")?.parse::<f64>().ok())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Rect;

    fn node(y: f64, height: f64) -> Node {
        Node {
            path: "html>body>div".into(),
            parent: Some("html>body".into()),
            tag: "div".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y,
                width: 1440.0,
                height,
            },
            style: Default::default(),
            before: None,
            after: None,
        }
    }

    #[test]
    fn converts_full_viewport_height() {
        let mut styles = Styles::new();
        normalize(
            &mut styles,
            &node(0.0, 900.0),
            &Viewport {
                width: 1440,
                height: 900,
                dpr: 1.0,
            },
        );
        assert_eq!(styles["height"], "100vh");
    }

    #[test]
    fn converts_header_offset_height() {
        let mut styles = Styles::new();
        normalize(
            &mut styles,
            &node(48.0, 852.0),
            &Viewport {
                width: 1440,
                height: 900,
                dpr: 1.0,
            },
        );
        assert_eq!(styles["height"], "calc(100vh - 48px)");
    }

    #[test]
    fn subtracts_content_box_padding() {
        let mut card = node(112.0, 456.0);
        card.style = Styles::from([
            ("box-sizing".into(), "content-box".into()),
            ("padding-top".into(), "0px".into()),
            ("padding-bottom".into(), "64px".into()),
        ]);
        let mut styles = card.style.clone();
        normalize(
            &mut styles,
            &card,
            &Viewport {
                width: 320,
                height: 568,
                dpr: 1.0,
            },
        );
        assert_eq!(styles["height"], "calc(100vh - 176px)");
    }
}
