mod animation_keyframes;
#[cfg(test)]
mod animation_order_tests;
mod animation_timing;
pub(crate) mod animations;
mod asset_extension;
mod asset_urls;
mod assets;
mod assets_remote;
mod attribute_sequences;
mod authored_css;
mod authored_css_index;
mod authored_css_rules;
mod authored_media;
#[cfg(test)]
#[path = "carousel_effect_tests.rs"]
mod carousel_effect_tests;
mod carousel_inference;
#[cfg(test)]
#[path = "carousel_inference_tests.rs"]
mod carousel_inference_tests;
mod component_identity;
mod css;
mod css_base;
mod css_base_style;
mod css_closure;
mod css_custom_properties;
mod css_declaration;
mod css_identifiers;
mod css_inheritance;
mod css_interactions;
mod css_layers;
mod css_layout;
mod css_paths;
mod css_pseudo;
#[cfg(test)]
mod css_pseudo_identity_tests;
#[cfg(test)]
mod css_pseudo_rule_tests;
mod css_rule_groups;
mod css_rule_split;
mod css_signature;
mod css_state_helpers;
mod css_values;
mod css_visual;
mod custom_properties;
mod custom_property_diff;
mod document;
#[cfg(test)]
#[path = "document_asset_tests.rs"]
mod document_asset_tests;
#[cfg(test)]
#[path = "document_css_tests.rs"]
mod document_css_tests;
#[cfg(test)]
#[path = "document_root_tests.rs"]
mod document_root_tests;
#[cfg(test)]
#[path = "flex_axis_tests.rs"]
mod flex_axis_tests;
mod generated_source;
mod inherited_styles;
mod initial_scroll;
mod interaction_labels;
mod interaction_scroll;
mod interactions;
mod jsx;
mod jsx_app;
mod jsx_attr_names;
mod jsx_attr_tables;
mod jsx_attrs;
mod jsx_host_props;
#[cfg(test)]
#[path = "jsx_host_props_tests.rs"]
mod jsx_host_props_tests;
mod jsx_markup;
mod jsx_markup_scan;
mod jsx_render;
mod jsx_render_spacing;
#[cfg(test)]
#[path = "jsx_render_spacing_tests.rs"]
mod jsx_render_spacing_tests;
mod jsx_state_changes;
mod jsx_state_existing;
mod jsx_state_generation;
mod jsx_state_overlay;
mod jsx_state_portals;
mod jsx_state_roots;
mod jsx_states;
mod jsx_text_entry;
mod jsx_variants;
#[cfg(test)]
#[path = "mount_tests.rs"]
mod mount_tests;
mod names;
#[cfg(test)]
mod names_tests;
mod project;
mod project_mount;
#[cfg(test)]
mod project_test_support;
#[cfg(test)]
mod project_text_entry_support;
mod responsive;
mod responsive_attributes;
mod responsive_geometry;
#[cfg(test)]
mod responsive_runtime_support;
#[cfg(test)]
mod responsive_runtime_tests;
mod root_reset;
mod roots;
mod runtime_sources;
mod scroll_state;
#[cfg(test)]
mod scroll_state_tests;
mod selector_list;
#[cfg(test)]
mod selector_list_tests;
mod source_css;
mod source_css_compact;
mod source_dedupe;
mod source_dedupe_support;
mod source_generated_blocks;
mod source_imports;
mod source_item_component;
mod source_item_dedupe;
mod source_item_name_words;
mod source_item_names;
#[cfg(test)]
mod source_item_tests;
mod source_split;
mod source_style_compact;
mod source_style_shards;
mod source_style_split;
mod source_style_support;
mod source_svg_assets;
#[cfg(test)]
#[path = "source_svg_assets_tests.rs"]
mod source_svg_assets_tests;
#[cfg(test)]
#[path = "source_svg_image_tests.rs"]
mod source_svg_image_tests;
#[cfg(test)]
#[path = "source_svg_name_tests.rs"]
mod source_svg_name_tests;
mod source_view_split;
mod stand_in;
mod startup_overlays;
mod startup_timeline;
#[cfg(test)]
#[path = "state_reversion_tests.rs"]
mod state_reversion_tests;
mod state_style_maps;
mod state_styles;
mod structural_css;
#[cfg(test)]
mod structural_tests;
mod style_delta;
#[cfg(test)]
#[path = "style_reversion_tests.rs"]
mod style_reversion_tests;

#[cfg(test)]
mod artifact_hermeticity_tests;
mod reemission;
mod structural_tree;
#[cfg(test)]
mod tests;
mod tree;
mod xml_namespaces;
#[cfg(test)]
#[path = "xml_namespaces_tests.rs"]
mod xml_namespaces_tests;

use crate::model::{BrowserCookie, Specification};
use anyhow::Result;
use std::{fs, path::Path};

pub async fn from_file(spec: &Path, out: &Path) -> Result<()> {
    let started = std::time::Instant::now();
    let timing = |phase: &str| {
        if std::env::var_os("RECREATE_TIMING").is_some() {
            eprintln!("generate_{phase}={:.3}s", started.elapsed().as_secs_f64());
        }
    };
    let mut bytes = fs::read(spec)?;
    timing("read");
    let mut specification: Specification = simd_json::serde::from_slice(&mut bytes)?;
    crate::interaction_surface::normalize(&mut specification);
    timing("parse");
    write_project(&specification, out, &[]).await?;
    timing("write");
    std::mem::forget(specification);
    std::mem::forget(bytes);
    Ok(())
}

pub use project::write_project;
