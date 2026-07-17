use crate::model::{Node, Styles, Viewport};

pub fn normalize(styles: &mut Styles, node: &Node, viewport: &Viewport) {
    let viewport_height = f64::from(viewport.height);
    let bottom = node.rect.y + node.rect.height;
    if node.rect.height < viewport_height * 0.7 || (bottom - viewport_height).abs() > 1.0 {
        return;
    }
    let height = if node.rect.y.abs() <= 1.0 {
        "100vh".into()
    } else if node.rect.y > 0.0 {
        format!("calc(100vh - {}px)", node.rect.y)
    } else {
        return;
    };
    styles.insert("height".into(), height);
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
}
