//! The single owner of the startup timeline: when the loading curtain arrives and how long
//! it stays.
//!
//! Capture measures one clock and `capture::state::merge_startup` splits it into two facts —
//! the interval before a curtain was first seen, and how long it then stayed up. Two
//! emitters replay that timeline: the overlay's CSS animation, which needs the phases
//! separately because `animation-delay` and `animation-duration` are separate properties,
//! and the runtime timer that holds the body scroll-lock, which needs their sum. Each had
//! re-derived the timeline from the raw fields, and they disagreed: the fragment wrote a
//! literal `0ms` delay and the timer counted only the curtain.
//!
//! So this module exposes the two renderings rather than the numbers. Nothing downstream
//! restates the arithmetic, which is what stops the CSS and the timer drifting apart again.

use crate::model::PageState;

/// Grace period after the curtain finishes before the body scroll-lock is released, so the
/// lock cannot end mid-fade on a machine slower than the one captured.
const SETTLE_MARGIN_MS: u64 = 500;

/// The two phases of one viewport's startup.
#[derive(Clone, Copy)]
pub(super) struct Timeline {
    delay_ms: u64,
    duration_ms: u64,
}

impl Timeline {
    pub(super) fn of(state: &PageState) -> Self {
        Self {
            delay_ms: state.startup_delay_ms,
            duration_ms: state.startup_duration_ms,
        }
    }

    /// The custom properties the overlay's animation reads, as an inline style body.
    ///
    /// The overlay's fill mode is `forwards`, which paints the base style during the delay
    /// rather than the first keyframe, so a real delay leaves the curtain genuinely hidden
    /// and non-interactive until the moment the original page showed it.
    pub(super) fn style_variables(self) -> String {
        let Self {
            delay_ms,
            duration_ms,
        } = self;
        format!(
            "\"--recreate-startup-delay\":\"{delay_ms}ms\",\
             \"--recreate-startup-duration\":\"{duration_ms}ms\""
        )
    }

    /// When the startup layer is over. Zero means the page had no startup layer at all,
    /// which is the only case with nothing to wait for.
    pub(super) fn settle_ms(self) -> u64 {
        match self.delay_ms + self.duration_ms {
            0 => 0,
            span => span + SETTLE_MARGIN_MS,
        }
    }
}

#[cfg(test)]
#[path = "startup_timeline_tests.rs"]
mod tests;
