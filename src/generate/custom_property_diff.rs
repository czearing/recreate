//! Reconciles a conditional band's custom properties against the unguarded base.
//!
//! The base is written outside every guard, so a band has to reproduce the
//! base's declaration *set*, not merely the values that differ: a property the
//! base declares and the band does not keeps applying below the breakpoint,
//! inside a guard that never mentions it. An absence is therefore something to
//! write, not something to omit.
//!
//! `initial` is that cancellation. On an unregistered custom property it sets
//! the guaranteed-invalid value, which is exactly the state a property is in
//! when no rule declares it, so every `var()` reading it takes its fallback as
//! it would have in the source. `unset` and the empty value both leave the
//! property declared and so do not reproduce an absence.
//!
//! Two limits are deliberate. Only custom properties reach here -- both
//! producers write `--`-prefixed declarations exclusively -- and `initial` does
//! not mean "undeclared" for a standard property, which would still inherit.
//! And a property registered through `@property` with an `initial-value`
//! resolves `initial` to that registered value rather than becoming invalid, so
//! the cancellation would be wrong for one; the generator drops `@property`
//! at-rules entirely today, so no registration survives into the output.

use std::collections::{BTreeMap, BTreeSet};

type Declarations = BTreeMap<String, String>;

const CANCELLED: &str = "initial";

pub fn against(base: &str, responsive: &str) -> String {
    let base = rules(base);
    let responsive = rules(responsive);
    union(&base, &responsive)
        .filter_map(|selector| {
            let declarations = reconcile(base.get(selector), responsive.get(selector));
            (!declarations.is_empty()).then(|| format!("{selector}{{{declarations}}}\n"))
        })
        .collect()
}

/// One question per property, asked of both sides at once, so a value that
/// changed, appeared, or disappeared each falls out of the same walk. Only a
/// pair that agrees stays silent, because the unguarded base already says it.
fn reconcile(base: Option<&Declarations>, band: Option<&Declarations>) -> String {
    let absent = Declarations::new();
    let base = base.unwrap_or(&absent);
    let band = band.unwrap_or(&absent);
    union(base, band)
        .filter_map(|property| {
            let value = match (base.get(property), band.get(property)) {
                (declared, Some(value)) if declared != Some(value) => value.as_str(),
                (Some(_), None) => CANCELLED,
                _ => return None,
            };
            Some(format!("{property}:{value};"))
        })
        .collect()
}

fn union<'a, T>(
    base: &'a BTreeMap<String, T>,
    band: &'a BTreeMap<String, T>,
) -> impl Iterator<Item = &'a String> {
    base.keys()
        .chain(band.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
}

/// Both producers write one rule per line and may repeat a selector, so the
/// lines are merged before either side is compared. Splitting to individual
/// declarations is what makes an absence visible at all: CSS cascades per
/// property, so a rule that merely lost one still reads as "changed" whole.
fn rules(css: &str) -> BTreeMap<String, Declarations> {
    let mut rules = BTreeMap::<String, Declarations>::new();
    for line in css.lines() {
        let Some((selector, block)) = line.split_once('{') else {
            continue;
        };
        let Some(block) = block.strip_suffix('}') else {
            continue;
        };
        let declarations = rules.entry(selector.into()).or_default();
        for declaration in block.split(';') {
            if let Some((property, value)) = declaration.split_once(':') {
                declarations.insert(property.trim().into(), value.trim().into());
            }
        }
    }
    rules
}

#[cfg(test)]
#[path = "custom_property_diff_tests.rs"]
mod tests;
