/// Whether an authored rule actually applies is a browser decision, not a parse decision.
///
/// `@supports` and `@container` expose their nested rules through the CSSOM even when
/// their condition does not match, so a walk that reads rule text alone records dead
/// declarations as authored ones. No API answers a container query, and evaluating each
/// at-rule family separately would need a new branch for every conditional at-rule the
/// platform adds. Instead every nested rule is re-emitted under its own at-rule prelude
/// chain with a sentinel custom property, and the browser decides: a rule is active when
/// at least one element it selects receives the sentinel. One code path covers `@media`,
/// `@supports`, `@container` and anything conditional that follows them.
pub const SOURCE: &str = include_str!("rule_activation_script.js");

#[cfg(test)]
#[path = "rule_activation_tests.rs"]
mod tests;
