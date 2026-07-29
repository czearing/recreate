use super::*;

#[test]
fn preserves_compact_control_width_when_it_fills_its_parent() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let control = node("button", 0.0, 28.0);
    let parent = node("div", 0.0, 28.0);
    let css = base_declarations(
        &control,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(css.contains("width:28px"));
}

#[test]
fn preserves_svg_graphic_width_when_it_fills_its_parent() {
    let viewport = Viewport {
        width: 320,
        height: 568,
        dpr: 1.0,
    };
    let graphic = node("rect", 0.0, 120.0);
    let parent = node("svg", 0.0, 120.0);
    let mut styles = graphic.style.clone();
    let base = node("rect", 0.0, 183.0);
    normalize_viewport_width(
        &mut styles,
        &graphic,
        Some(&parent),
        &viewport,
        Some((
            &base,
            &Viewport {
                width: 1920,
                height: 1080,
                dpr: 1.0,
            },
        )),
    );
    assert_eq!(styles.get("width").map(String::as_str), Some("120px"));
}

#[test]
fn preserves_compact_role_image_width_when_it_fills_its_parent() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut avatar = node("span", 0.0, 28.0);
    avatar.attributes.insert("role".into(), "img".into());
    let parent = node("div", 0.0, 28.0);
    let css = base_declarations(
        &avatar,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(css.contains("width:28px"));
}

#[test]
fn preserves_intrinsic_svg_aspect_width() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let image = node("svg", 0.0, 174.5);
    let parent = node("div", 0.0, 174.5);
    let css = base_declarations(
        &image,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(css.contains("width:174.5px"));
}
