use super::authored_condition_chain::for_each_rule;
use super::authored_conditions_measured::Measured;
use super::authored_css_index::Index;
use super::selector_scope::Scope;
use crate::model::Node;
use std::collections::{BTreeSet, HashSet};

/// One authored condition rule rewritten onto generated classes, kept in parts.
///
/// The parts stay apart because the emitter merges rules that share a condition and a
/// declaration block onto one selector list, and a rule already spelled out as text cannot be
/// merged without being read back. A page whose sheet wraps every rule in the identity condition
/// reaches that emitter with one copy of each reset per element.
pub struct Emitted {
    /// The chain of conditions spelled as the text that opens it, outermost first.
    pub opening: String,
    pub selector: String,
    pub declarations: String,
}

impl Emitted {
    /// The rule as CSS. A prelude is by definition the text before a brace, so the braces the
    /// chain was joined on are its own and counting them recovers how many to close.
    pub fn text(&self) -> String {
        format!(
            "{}{{{}{{{}}}{}",
            self.opening,
            self.selector,
            self.declarations,
            "}".repeat(self.opening.matches('{').count() + 1)
        )
    }
}

/// The authored condition rules this node keeps, and the compounds their selectors name.
///
/// A compound is reported only when a rule survives deduplication, so a page with no
/// authored condition rule reports none and gains no markers.
///
/// The authored rules are only half of what a node owes a condition. A conditional block
/// that declares a custom property, or one that reaches this element through a selector this
/// stage cannot rewrite, still decided properties here — the capture measured which — and
/// none of them is spelled in any text below. Those are added to the band the chain already
/// emits, or published on the node's own generated class where the chain emits none, so that
/// every property [`restore_unconditional`] takes out of the base rule is put back by
/// something — and only those, since both stages read one answer from the same index.
pub fn rules(
    node: &Node,
    scope: &Scope<'_>,
    rules: &[String],
    index: &Index<'_>,
    measured: &Measured,
    compounds: &mut BTreeSet<String>,
) -> Vec<Emitted> {
    let mut output = Vec::new();
    let mut seen = HashSet::new();
    for rule in rules {
        for_each_rule(rule, &mut |conditions, selectors, declarations| {
            let Some(scoped) = super::selector_list::static_members(selectors)
                .find_map(|member| scope.rewrite(&member, node))
            else {
                return;
            };
            let emitted = Emitted {
                opening: conditions.opening(),
                selector: scoped.selector,
                declarations: declarations.trim().to_string(),
            };
            if seen.insert(emitted.text()) {
                output.push(emitted);
                compounds.extend(scoped.compounds.iter().cloned());
            }
        });
    }
    restore_measured(node, scope, index, measured, &mut output);
    output
}

/// The overrides the engine credited to a chain that no rewritten rule above already states,
/// at the value the capture measured with that condition in force.
///
/// The comparison is against what the base rule will finally say, not against what the node
/// measured, because [`restore_unconditional`] has by then replaced the override there with
/// the arm below it — and a property that rule drops altogether is one the author declared
/// nowhere, a used value the withdrawal merely reflowed, which layout re-derives from
/// whatever did change.
fn restore_measured(
    node: &Node,
    scope: &Scope<'_>,
    index: &Index<'_>,
    measured: &Measured,
    output: &mut Vec<Emitted>,
) {
    let names = measured
        .at(&node.path)
        .flat_map(|(_, properties)| properties.keys())
        .filter(|property| node.style.contains_key(*property))
        .cloned()
        .collect();
    let arms = super::authored_conditions_base_arm::arms(node, index, names);
    for (opening, properties) in measured.at(&node.path) {
        let mut band = output
            .iter_mut()
            .find(|emitted| emitted.opening == *opening)
            .map(|emitted| (emitted.declarations.contains(':'), emitted));
        let missing: Vec<_> = properties
            .iter()
            .filter(|(property, value)| match arms.get(*property) {
                Some(super::authored_conditions_base_arm::Arm::Drop) => false,
                Some(super::authored_conditions_base_arm::Arm::Value(base)) => base != *value,
                _ => node.style.get(*property).is_some_and(|base| base != *value),
            })
            .filter(|(property, _)| {
                band.as_ref().is_none_or(|(_, emitted)| {
                    !super::css_declaration::parsed(&emitted.declarations).any(|(name, value)| {
                        !matches!(
                            super::shorthand::claim(
                                index.shorthands(),
                                &emitted.declarations,
                                name,
                                value,
                                property,
                            ),
                            super::shorthand::Claim::Elsewhere
                        )
                    })
                })
            })
            .map(|(property, value)| format!("{property}:{value};"))
            .collect();
        if missing.is_empty() {
            continue;
        }
        match band.as_mut() {
            Some((stated, emitted)) => {
                if *stated && !emitted.declarations.trim_end().ends_with(';') {
                    emitted.declarations.push(';');
                }
                emitted.declarations.push_str(&missing.concat());
            }
            None => {
                let Some(class) = scope.class(node) else {
                    continue;
                };
                output.push(Emitted {
                    opening: opening.clone(),
                    selector: format!(".{class}"),
                    declarations: missing.concat(),
                });
            }
        }
    }
}

/// Each republished chain paired with the properties it decided that this node's own
/// generated class carries.
///
/// The intersection with [`Node::style`] is the whole of the reach test. A condition decides
/// properties on elements far outside anything the emitter can rewrite, but only a property
/// the node bakes can be wrong in the output, and only a property the node bakes can be put
/// back on the class that bakes it — so the same one test bounds both directions.
pub(super) fn withdrawable(
    node: &Node,
) -> impl Iterator<Item = (&String, impl Iterator<Item = &String>)> {
    node.condition_decided.iter().map(|(opening, properties)| {
        (
            opening,
            properties
                .iter()
                .filter(|property| node.style.contains_key(*property)),
        )
    })
}

#[cfg(test)]
#[path = "authored_conditions_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "authored_conditions_base_arm_tests.rs"]
mod base_arm_tests;
