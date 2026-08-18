/// The one place that decides which tree scopes a capture reaches, and the one way it puts a
/// rule in force across all of them for the duration of a read.
///
/// A rule reaches the tree scope whose stylesheet holds it and no other. Every stage that
/// declares something about the page in order to measure it — the user-agent baseline, the
/// transition suspension — therefore has a reach, and a stage that installs its rule in the
/// document alone measures a shadow tree as though it had declared nothing. The failure is
/// silent, because the read still succeeds and returns the page's own live values.
///
/// Owned here rather than at each reader so that "reaches every tree scope" is decided once. It
/// was previously derived three times, and a scope one derivation entered was not necessarily a
/// scope the others did.
pub const SOURCE: &str = include_str!("scoped_rules.js");

#[cfg(test)]
#[path = "scoped_rules_tests.rs"]
mod tests;
