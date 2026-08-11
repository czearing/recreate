//! The difference itself, below any emitter: what two style maps alone can say.
use super::{REVERTED, changed_styles};
use crate::model::Styles;

/// A reversion is spelled `revert`: the capture defines the pruned set by applying
/// `all: revert`, so `revert` re-runs the same query. `initial` would give `display:
/// inline` for a div and `unset` never revives the user-agent origin.
#[test]
fn spells_a_reversion_with_the_keyword_the_capture_measured_with() {
    let base = Styles::from([("display".into(), "flex".into())]);
    let narrow = Styles::new();
    let mut changed = changed_styles(&base, &narrow);
    super::append_reversions(&mut changed, &base, &narrow);
    assert_eq!(changed.get("display").map(String::as_str), Some(REVERTED));
    assert_eq!(REVERTED, "revert");
}
