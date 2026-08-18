//! Page states and nodes assembled for tests of the interaction stage, so the shape of an
//! empty state is written once rather than in every file that needs one.
pub fn empty_state() -> crate::model::PageState {
    crate::model::PageState {
        url: String::new(),
        title: String::new(),
        viewport: crate::model::Viewport::default(),
        dom: Default::default(),
        capture_blockers: Vec::new(),
        nodes: Vec::new(),
        startup_nodes: Vec::new(),
        startup_delay_ms: 0,
        startup_duration_ms: 0,
        animations: Vec::new(),
        state_styles: Vec::new(),
        attribute_sequences: Vec::new(),
        css_rules: Vec::new(),
        css_shorthands: Default::default(),
        asset_urls: Vec::new(),
        asset_data: Default::default(),
    }
}

pub fn empty_node(path: &str) -> crate::model::Node {
    crate::model::Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        path: path.into(),
        parent: path.rsplit_once('>').map(|(parent, _)| parent.into()),
        tag: path
            .rsplit_once('>')
            .map_or(path, |(_, node)| node)
            .split(':')
            .next()
            .unwrap_or("div")
            .into(),
        text: String::new(),
        attributes: Default::default(),
        rect: crate::model::Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        style: Default::default(),
        disabled: false,
        rtl: false,
        ..Default::default()
    }
}
