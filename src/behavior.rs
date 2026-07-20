#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TriggerKey {
    pub path: String,
    pub tag: String,
    pub label: String,
    pub occurrence: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TriggerCandidate<'a> {
    pub path: &'a str,
    pub tag: &'a str,
    pub label: &'a str,
}

pub fn resolve_trigger<'a>(
    key: &TriggerKey,
    candidates: &'a [TriggerCandidate<'a>],
) -> Option<&'a str> {
    if let Some(exact) = candidates.iter().find(|candidate| {
        candidate.path == key.path && candidate.tag == key.tag && candidate.label == key.label
    }) {
        return Some(exact.path);
    }
    let mut matches = candidates
        .iter()
        .filter(|candidate| candidate.tag == key.tag && candidate.label == key.label);
    if let Some(occurrence) = key.occurrence {
        return matches.nth(occurrence).map(|candidate| candidate.path);
    }
    let first = matches.next()?;
    matches.next().is_none().then_some(first.path)
}

#[cfg(test)]
#[path = "behavior_tests.rs"]
mod tests;
