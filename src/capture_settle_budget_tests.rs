//! The budget the settle ceiling grants is only real if the caller is still listening when
//! it expires, so the two constants that decide that are pinned against each other here.

/// The ceiling is a grant: a page that never goes quiet is still meant to be captured once
/// it expires. That outcome only reaches the caller if the caller is still listening, so the
/// transport deadline has to outlast the longest budget any injected script is given. These
/// two numbers were declared independently and were equal, which made the grant unreachable
/// by construction — the probe resolved at the instant the client gave up, so every page that
/// needed the full budget failed rather than being captured late.
#[test]
fn the_transport_outlasts_the_budget_it_is_waiting_on() {
    let ceiling = std::time::Duration::from_millis(crate::capture_settle::READY_CEILING_MS);
    assert!(
        crate::capture_settle::TRANSPORT_DEADLINE > ceiling,
        "a page granted {ceiling:?} cannot answer a caller that leaves after {:?}",
        crate::capture_settle::TRANSPORT_DEADLINE
    );
}
