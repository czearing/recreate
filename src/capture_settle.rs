use std::time::Duration;

/// The repeated geometry readings a ready page must record before capture.
const STABLE_POLLS: u32 = 2;
/// How often geometry is re-read, so `STABLE_POLLS` repeats span a known stretch of time.
pub(crate) const POLL_INTERVAL_MS: u64 = 250;
/// The longest a ready page waits for its geometry to stop changing.
const STABLE_CEILING_MS: u64 = 8_000;

/// Counts how many consecutive polls have seen the same geometry.
///
/// Geometry stability and page readiness are two different measurements of the same page,
/// and nothing makes one depend on the other. Resetting this count until the page reported
/// ready ran the two windows in series, so every capture paid for both one after the other
/// even though the readings needed are gathered at the same time. Both still have to hold
/// together at the moment of capture; they are simply no longer queued behind each other.
pub(crate) fn next_stable(previous: &str, signature: &str, stable: u32) -> u32 {
    if !signature.is_empty() && signature == previous {
        stable + 1
    } else {
        0
    }
}

/// Decides when a ready page may be captured.
///
/// Layout keeps changing after the last resource resolves, so readiness alone snapshots a
/// half-laid-out page. Capture waits for the geometry signature to repeat, which measures
/// the page. It does not additionally wait out a fixed second: elapsed time is not evidence
/// of anything, and the repeats already imply the time they took to collect. The ceiling
/// releases a page whose geometry never repeats, such as one running a looping animation.
pub(crate) fn capture_ready(elapsed: Duration, stable: u32) -> bool {
    stable >= STABLE_POLLS || elapsed >= Duration::from_millis(STABLE_CEILING_MS)
}

#[cfg(test)]
mod tests {
    use super::{POLL_INTERVAL_MS, STABLE_CEILING_MS, STABLE_POLLS, capture_ready, next_stable};
    use std::time::Duration;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn a_ready_page_whose_layout_still_moves_is_not_captured() {
        assert!(
            !capture_ready(ms(1_200), 0),
            "got true: polls={STABLE_POLLS} ceiling={STABLE_CEILING_MS}"
        );
        assert!(!capture_ready(ms(3_000), 1));
    }

    #[test]
    fn a_ready_page_with_repeated_geometry_is_captured() {
        assert!(capture_ready(ms(1_200), 2));
    }

    /// Replaces a test that asserted a one-second floor using `capture_ready(900ms, 5)`, an
    /// input the poll loop cannot produce: five repeats cost at least
    /// `5 * POLL_INTERVAL_MS`, so the floor could only ever delay a page whose geometry had
    /// already been proven unchanged. Stability is evidence; elapsed time is not.
    #[test]
    fn stability_is_measured_by_repeats_and_not_by_elapsed_time() {
        let earliest = ms(u64::from(STABLE_POLLS) * POLL_INTERVAL_MS);
        assert!(
            capture_ready(earliest, STABLE_POLLS),
            "delayed a page whose geometry had already repeated {STABLE_POLLS} times"
        );
        assert!(
            !capture_ready(ms(STABLE_CEILING_MS - 1), STABLE_POLLS - 1),
            "time alone stood in for evidence below the ceiling"
        );
    }

    #[test]
    fn a_page_that_never_repeats_is_released_at_the_ceiling() {
        assert!(capture_ready(ms(STABLE_CEILING_MS), 0));
    }

    /// Geometry that has not moved counts even before the page reports ready, so the two
    /// settle measurements overlap instead of queueing.
    #[test]
    fn unchanged_geometry_counts_while_the_page_is_still_becoming_ready() {
        assert_eq!(next_stable("a:1", "a:1", 0), 1);
        assert_eq!(next_stable("a:1", "a:1", 1), 2);
    }

    /// The inverse. Moving geometry, and a poll that read no geometry at all, must both
    /// discard the evidence collected so far rather than let a stale count stand.
    #[test]
    fn changed_or_absent_geometry_discards_the_count() {
        assert_eq!(next_stable("a:1", "a:2", 5), 0);
        assert_eq!(next_stable("", "", 5), 0);
    }
}
