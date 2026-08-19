//! Every override a re-published condition was measured to carry, gathered from every state.
//!
//! A condition's arms are measured only where it holds, and the base width holds one of them.
//! `@media (max-width: 700px)` is false at every width the base state was captured at, so its
//! override appears in no reading that state took — while the sweep, which visits 390 and 320,
//! measures it there and nowhere else. The band for it therefore cannot be built from one
//! state's evidence, and the viewport bands are not its carrier either: they are quantised to
//! the widths the capture sampled, so an author's breakpoint between two samples is wrong for
//! every width in between.
//!
//! Gathered once for the whole page rather than per node, because the states are walked once
//! either way and a node's evidence is a lookup by the path it already carries.

use crate::model::PageState;
use std::collections::BTreeMap;

/// Override values by element path, then by the chain that decided them, then by property.
#[derive(Default)]
pub struct Measured(BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>);

impl Measured {
    pub fn new(states: &[PageState]) -> Self {
        let mut measured: BTreeMap<_, BTreeMap<_, BTreeMap<_, _>>> = BTreeMap::new();
        for state in states {
            for node in &state.nodes {
                for (opening, properties) in &node.condition_decided {
                    for property in properties {
                        let Some(value) = node.style.get(property) else {
                            continue;
                        };
                        // The base state is first, so where two widths measured one condition
                        // differently — an override spelled in viewport units — the width the
                        // unconditional rules were written against wins.
                        measured
                            .entry(node.path.clone())
                            .or_default()
                            .entry(opening.clone())
                            .or_default()
                            .entry(property.clone())
                            .or_insert_with(|| value.clone());
                    }
                }
            }
        }
        Self(measured)
    }

    pub fn at(&self, path: &str) -> impl Iterator<Item = (&String, &BTreeMap<String, String>)> {
        self.0.get(path).into_iter().flatten()
    }
}

/// The same gather over a bare node list, for cases that build their scene as nodes rather
/// than as captured states.
#[cfg(test)]
pub(super) fn from_nodes(nodes: &[crate::model::Node]) -> Measured {
    Measured::new(&[PageState {
        nodes: nodes.to_vec(),
        ..Default::default()
    }])
}
