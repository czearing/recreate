use crate::model::Checkpoint;
use serde_json::Value;

const DOMAINS: [&str; 6] = [
    "interaction",
    "structure",
    "accessibility",
    "motion",
    "geometry",
    "style",
];

pub fn matches_clean(traced: &Checkpoint, clean: &Checkpoint) -> bool {
    difference(traced, clean).is_none()
}

pub fn difference(
    traced: &Checkpoint,
    clean: &Checkpoint,
) -> Option<(&'static str, String, Value, Value)> {
    DOMAINS.iter().find_map(|domain| {
        let traced = &traced.domains[*domain];
        let clean = &clean.domains[*domain];
        (traced.digest != clean.digest).then(|| {
            let (path, traced, clean) =
                crate::compare_difference::between(&traced.value, &clean.value);
            (*domain, path, traced, clean)
        })
    })
}
