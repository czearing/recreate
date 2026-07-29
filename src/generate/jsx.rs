pub use super::jsx_app::app;
pub use super::jsx_render::component;
pub(super) use super::jsx_render::{render, render_children};

#[cfg(test)]
use super::tree::Components;
#[cfg(test)]
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "jsx_tests.rs"]
mod tests;
