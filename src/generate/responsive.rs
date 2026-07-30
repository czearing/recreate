#[path = "responsive_flex.rs"]
mod flex;
#[path = "responsive_node_rules.rs"]
mod node_rules;
#[path = "responsive_output.rs"]
mod output;
#[path = "responsive_rules.rs"]
mod rules;

pub use output::{base_declarations, base_declarations_indexed, output_declarations};
pub use rules::append_filtered;
pub(super) use rules::{band, media_rule};

#[cfg(test)]
pub(super) use flex::{constrained_by_flex_chain, shrunk_flex_item};
#[cfg(test)]
pub(super) use node_rules::changed_styles;

#[cfg(test)]
fn normalize_viewport_width(
    styles: &mut crate::model::Styles,
    node: &crate::model::Node,
    parent: Option<&crate::model::Node>,
    viewport: &crate::model::Viewport,
    base: Option<(&crate::model::Node, &crate::model::Viewport)>,
) {
    super::responsive_geometry::normalize(styles, node, parent, viewport, base);
}

#[cfg(test)]
#[path = "responsive_tests.rs"]
mod tests;
