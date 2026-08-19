/// The one reader of selector text in the capture.
///
/// Three stages ask structural questions about an authored selector: the state capture, the
/// activation probe and the generated-box scan. Each used to answer them with `split(',')`
/// and a regex, and each was wrong for the same input — a comma nested in a functional
/// pseudo-class or a quoted attribute value — because balanced delimiters are not a regular
/// language and no pattern without a depth counter reads them.
///
/// The failure is silent by construction. A fragment cut out of the middle of a selector is
/// usually still a selector, so it either matches a population the author never named or
/// throws where the throw is swallowed and the rule leaves no record at all.
///
/// Declared here, ahead of every reader, so a selector cannot be read two ways.
pub const SOURCE: &str = include_str!("selector_scan.js");

#[cfg(test)]
#[path = "selector_scan_tests.rs"]
mod tests;
