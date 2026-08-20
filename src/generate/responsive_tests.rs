use super::*;
use crate::model::{Attributes, Node, Rect, Styles, Viewport};
use std::collections::{HashMap, HashSet};

pub(super) fn node(tag: &str, x: f64, width: f64) -> Node {
    let mut attributes = Attributes::new();
    if tag == "root" {
        attributes.insert("id".into(), "root".into());
    }
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: tag.into(),
        parent: None,
        tag: if tag == "root" { "div" } else { tag }.into(),
        text: String::new(),
        attributes,
        rect: Rect {
            x,
            y: 0.0,
            width,
            height: 40.0,
        },
        style: Styles::from([("width".into(), format!("{width}px"))]),
        ..Default::default()
    }
}

#[path = "responsive_anchor_tests.rs"]
mod anchor_tests;
#[path = "responsive_box_tests.rs"]
mod box_tests;
#[path = "responsive_flex_tests.rs"]
mod flex_tests;
#[path = "responsive_intrinsic_tests.rs"]
mod intrinsic_tests;
#[path = "responsive_layout_tests.rs"]
mod layout_tests;
#[path = "responsive_output_reset_tests.rs"]
mod output_reset_tests;
#[path = "responsive_output_tests.rs"]
mod output_tests;
#[path = "responsive_provenance_tests.rs"]
mod provenance_tests;
#[path = "responsive_root_tests.rs"]
mod root_tests;
