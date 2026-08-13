//! What orders the reading, and at what precision its measurements are kept.
//!
//! These pin the two properties that cannot be observed from a recorded phase after the fact:
//! that the reader is armed so it cannot lose a race with the page's own first frame, and
//! that the span it reports survives a recapture unchanged.

use super::{grain, source};

/// The reader is ordered by frame precedence, not by a timer or a lifecycle event.
///
/// It must re-arm rather than read the first frame it sees, because the earliest frames land
/// while the document is still parsing and there is nothing to read. It must not wait for
/// `DOMContentLoaded` either: a page can paint and mutate across two frames before parsing
/// ends, and a reader armed there records the page after the phase it exists to catch — one
/// capture in three, measured. Re-arming from inside its own frame callback is what keeps it
/// ahead of anything the page schedules.
#[test]
fn the_reader_re_arms_until_the_page_has_something_to_read() {
    let source = source();

    assert!(
        source.contains("requestAnimationFrame"),
        "frame order is the only ordering guarantee available"
    );
    assert!(
        source.contains("if(!painted())return arm()"),
        "a frame with nothing on it must be waited through, not recorded"
    );
    assert!(
        source.contains("document.body.children.length>0"),
        "the wait ends on the page having content, not on a duration"
    );
    assert!(
        source.contains("document.readyState!=='loading'"),
        "a page whose body stays empty must still be read once, not watched forever"
    );
    assert!(
        !source.contains("DOMContentLoaded"),
        "parsing can finish after the page has already mutated across two frames"
    );
    assert!(
        source.contains("performance.now()"),
        "the elapsed time is read in the page, so no round trip is counted as page time"
    );
    assert!(
        !source.contains("fetch("),
        "the reading must not put requests in flight that the settle gate would wait on"
    );
}

/// Two captures of an unchanged page must produce the same source. The measured span is the
/// only quantity in the startup layer that comes from a clock, so it is the only one that can
/// make a recapture differ; the jitter observed between two runs of the same scene was 21ms.
/// Rounding is not cosmetic here — without it every sweep reports this page as changed and a
/// real regression cannot be told from the noise.
#[test]
fn spans_that_differ_only_by_measurement_jitter_record_the_same_number() {
    assert_eq!(
        grain(577),
        grain(556),
        "21ms of jitter must not move the value"
    );
    assert_eq!(grain(398), grain(397));
    assert_eq!(grain(0), 0, "no phase stays no phase");
}

/// Rounding must not collapse the scale. A phase that is genuinely twice as long has to stay
/// twice as long, or the replay stops being a replay.
#[test]
fn spans_that_genuinely_differ_stay_apart() {
    assert!(grain(1200) > grain(800));
    assert_eq!(grain(1200) - grain(800), 400);
    assert_eq!(
        grain(60),
        100,
        "a phase shorter than the grain still happened"
    );
}
