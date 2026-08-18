use super::authored_condition_chain::for_each_rule;
use super::authored_css_index::Index;
use super::selector_scope::Scope;
use crate::model::{Node, Styles};
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
pub fn rules(
    node: &Node,
    scope: &Scope<'_>,
    rules: &[String],
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
    output
}

/// The value the base rule owes when a document-answered condition supplied the one measured.
///
/// A capture reads each element once per sampled viewport, so every declaration it records is
/// the branch the conditions happened to be on. The prelude is re-emitted above, which puts
/// that branch back wherever the condition holds — but nothing had removed it from the base
/// rule, so the recreation stated the override twice and stated the arm below the breakpoint
/// nowhere, painting the override at every width.
///
/// The condition's own declaration is the proof. `@media` and `@container` add no specificity,
/// so a conditional declaration is the computed value exactly while its condition holds; a
/// value equal to the sample therefore reports the engine's own answer, at a width the capture
/// really visited, without this stage evaluating a single media feature. Where it disagrees the
/// condition was false, or that declaration lost the cascade, and the measured value stands.
///
/// What replaces it is the unconditional cascade's own last word, or nothing where the author
/// wrote none — below the breakpoint the element takes its inherited or initial value, which
/// the recreation re-produces by saying nothing. No width is consulted, so this is equally the
/// answer for a container query, whose condition no viewport can settle at all.
pub fn restore_unconditional(styles: &mut Styles, node: &Node, index: &Index<'_>) {
    let matched = index.conditional_declarations(node);
    if matched.is_empty() {
        return;
    }
    let mut withdraw = BTreeSet::new();
    for declarations in matched.iter() {
        for (name, value) in super::css_declaration::parsed(declarations) {
            // Read the same way the emission resolves it, so a shorthand override — which
            // reaches the base rule split into one declaration per edge — is recognised on
            // exactly the edges it actually set.
            for (name, value) in super::authored_css_rules::physical_property(node, name)
                .into_declarations(name, value)
            {
                // The proof, and the whole of it: this property's own sample equals what the
                // condition declares for it. A value merely present elsewhere in the node's
                // style is a coincidence, not evidence. Asked of every longhand the name
                // stands for, because a capture enumerates longhands and a name the author
                // shortened matches none of them; the block travels with it, because how the
                // engine divided a shorthand is a fact about the block it was written in.
                let shorthands = index.shorthands();
                withdraw.extend(super::shorthand::measured(
                    shorthands,
                    declarations,
                    &node.style,
                    &name,
                    &value,
                ));
            }
        }
    }
    if withdraw.is_empty() {
        return;
    }
    // Asked once for the whole set rather than once per property: the unconditional cascade
    // is resolved by walking this node's rules, and walking them per declaration is what a
    // page with five figures of both costs most.
    let unconditional = index.unconditional_values(node, &withdraw);
    for name in withdraw {
        match unconditional.get(&name) {
            // Declared, and divided into a share the engine itself could not settle. The
            // measured value is the only one this can state, so the withdrawal is dropped
            // rather than completed with a value nothing supports.
            Some(None) => (),
            Some(Some(value)) => {
                styles.insert(name, value.clone());
            }
            None => {
                styles.remove(&name);
            }
        }
    }
}

#[cfg(test)]
#[path = "authored_conditions_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "authored_conditions_base_arm_tests.rs"]
mod base_arm_tests;
