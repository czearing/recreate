use super::*;

#[test]
fn removes_measured_width_when_border_box_fills_parent_content() {
    let viewport = Viewport {
        width: 768,
        height: 900,
        dpr: 1.0,
    };
    let mut parent = node("parent", 4.0, 760.0);
    parent.style.extend([
        ("box-sizing".into(), "content-box".into()),
        ("width".into(), "758px".into()),
        ("border-left-width".into(), "1px".into()),
        ("border-right-width".into(), "1px".into()),
    ]);
    let mut child = node("child", 5.0, 758.0);
    child.style.extend([
        ("box-sizing".into(), "content-box".into()),
        ("width".into(), "714px".into()),
        ("padding-left".into(), "22px".into()),
        ("padding-right".into(), "22px".into()),
    ]);
    let css = base_declarations(
        &child,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(!css.contains("width:714px"));
}

#[test]
fn removes_measured_width_when_parent_uses_border_shorthand() {
    let viewport = Viewport {
        width: 1920,
        height: 1080,
        dpr: 1.0,
    };
    let mut parent = node("parent", 465.0, 980.0);
    parent.style.extend([
        ("box-sizing".into(), "border-box".into()),
        ("width".into(), "980px".into()),
        ("border".into(), "1px solid rgb(0, 0, 0)".into()),
    ]);
    let mut child = node("child", 466.0, 978.0);
    child.style.extend([
        ("box-sizing".into(), "border-box".into()),
        ("width".into(), "978px".into()),
        ("padding-left".into(), "16px".into()),
        ("padding-right".into(), "16px".into()),
    ]);
    let css = base_declarations(
        &child,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(!css.contains("width:978px"));
}

#[test]
fn removes_measured_height_for_evidence_backed_content_reflow() {
    let viewport = Viewport {
        width: 768,
        height: 900,
        dpr: 1.0,
    };
    let mut card = node("button", 0.0, 369.0);
    card.style.insert("height".into(), "245px".into());
    let css = base_declarations(
        &card,
        None,
        &viewport,
        &Default::default(),
        &[],
        true,
        false,
    );
    assert!(!css.contains("height:245px"));
}

#[test]
fn resets_captured_width_when_child_becomes_parent_filling() {
    let wide = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let narrow = Viewport {
        width: 768,
        height: 900,
        dpr: 1.0,
    };
    let base = node("child", 204.0, 1076.0);
    let mut current = node("child", 5.0, 758.0);
    current.style.insert("width".into(), "714px".into());
    let mut parent = node("parent", 4.0, 760.0);
    parent.style.extend([
        ("box-sizing".into(), "content-box".into()),
        ("width".into(), "758px".into()),
        ("border-left-width".into(), "1px".into()),
        ("border-right-width".into(), "1px".into()),
    ]);
    let mut changed = changed_styles(&base.style, &current.style);
    crate::generate::responsive_geometry::normalize(
        &mut changed,
        &current,
        Some(&parent),
        &narrow,
        Some((&base, &wide)),
    );
    assert_eq!(changed.get("width").map(String::as_str), Some("auto"));
}
