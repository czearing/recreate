//! The authored before-change style, which no animation record can carry.
//!
//! A transition that runs on an element's first render has no previous computed style to
//! start from, so `@starting-style` supplies one. The capture reads transitions through
//! `getKeyframes()`, and that report is trustworthy only for a property with no keyword
//! form: `opacity` arrives as the authored `0`, while `translate`, `transform`, `rotate`,
//! `scale` and `filter` arrive as `none`. `none` is also a legitimate authored value, so
//! nothing downstream can tell a lost start from a real identity, and the entry motion
//! silently flattens to whatever the resting style already is.
//!
//! The authored rule is the only surviving record of the value, and the capture does keep
//! it. Reading it out of either authoring form belongs to `starting_style`; this module is
//! the one place it is joined to an animation. It supplies a frame the animation API could
//! not report and nothing else — in particular the declarations never reach an element's
//! resting style, because a before-change style is by construction the value the element is
//! *not* at once the change has happened.
//!
//! Re-emitting the rule instead would not work: every authored declaration reaches an
//! element through a generated class, so a copied `@starting-style` block keeps its author
//! selector and matches nothing in the emitted markup.

use crate::model::{Animation, Node, Styles};
use serde_json::Value;
use std::collections::HashMap;

/// Every element's authored before-change style, keyed by the path an animation targets.
#[derive(Default)]
pub(crate) struct BeforeChange {
    by_target: HashMap<String, Styles>,
}

impl BeforeChange {
    pub(crate) fn new(rules: &[String], nodes: &[Node]) -> Self {
        let blocks = super::starting_style::declarations(rules);
        if blocks.is_empty() {
            return Self::default();
        }
        let mut by_target: HashMap<String, Styles> = HashMap::new();
        for node in nodes {
            let mut declared = Styles::new();
            for (selectors, declarations) in &blocks {
                if !super::authored_css::directly_targets_node(selectors, node) {
                    continue;
                }
                for (name, value) in super::css_declaration::parsed(declarations) {
                    declared.insert(name.into(), value.into());
                }
            }
            if !declared.is_empty() {
                by_target.insert(node.path.clone(), declared);
            }
        }
        Self { by_target }
    }

    /// The animations with every opening frame the API could not report restored.
    ///
    /// Only a value the report could not distinguish from a lost one is replaced. Where the
    /// API named a value, it measured the running transition and is the better authority —
    /// the authored rule states what the cascade was asked for, not what the frame reached.
    pub(crate) fn seed(&self, animations: &[Animation]) -> Vec<Animation> {
        if self.by_target.is_empty() {
            return animations.to_vec();
        }
        animations
            .iter()
            .map(|animation| {
                let mut animation = animation.clone();
                let Some(declared) = self.by_target.get(&animation.target) else {
                    return animation;
                };
                if let Some(frame) = opening_frame(&mut animation) {
                    for (key, value) in frame.iter_mut() {
                        if !unreported(value) {
                            continue;
                        }
                        if let Some(authored) =
                            declared.get(&super::animation_keyframes::kebab(key))
                        {
                            *value = Value::String(authored.clone());
                        }
                    }
                }
                animation
            })
            .collect()
    }
}

/// The frame the transition starts from, which is the earliest recorded place rather than
/// the first listed one, so a record whose frames arrive out of order is still seeded at its
/// true beginning instead of somewhere along the movement.
fn opening_frame(animation: &mut Animation) -> Option<&mut serde_json::Map<String, Value>> {
    let earliest = animation
        .keyframes
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| offset(left).total_cmp(&offset(right)))
        .map(|(index, _)| index)?;
    animation.keyframes.get_mut(earliest)?.as_object_mut()
}

fn offset(frame: &Value) -> f64 {
    frame["computedOffset"]
        .as_f64()
        .or_else(|| frame["offset"].as_f64())
        .unwrap_or(0.0)
}

/// Whether the animation API declined to name this value.
///
/// `none` is the initial value of every property whose start is lost this way, and the
/// report carries it in place of the authored start. It is also a value an author can
/// write, which is exactly why the loss is silent — and why substituting the authored
/// declaration is right in both readings: where the author wrote `none`, the authored value
/// and the reported value agree.
fn unreported(value: &Value) -> bool {
    value.as_str() == Some("none")
}

#[cfg(test)]
#[path = "before_change_tests.rs"]
mod tests;
