//! The single owner of how a first-paint phase is replayed: when it was on screen, and where.
//!
//! Capture measures one clock and `first_paint::merge` splits it into two facts — the
//! interval before the phase was read, and how long it then stayed. Two emitters replay that
//! timing: the overlay's CSS animation, which needs the phases separately because
//! `animation-delay` and `animation-duration` are separate properties, and the runtime timer
//! that holds the body scroll-lock, which needs their sum. Each had re-derived it from the
//! raw fields, and they disagreed: the fragment wrote a literal `0ms` delay and the timer
//! counted only the curtain.
//!
//! Placement is here for the same reason. The replayed layer is portalled to `document.body`
//! and so is torn out of the position its markup gave it; it can only land in the right place
//! if it is told where the phase actually was. That used to be assumed rather than measured —
//! a hardcoded `inset: 0; width: 100vw; height: 100vh` — which is right for a splash curtain
//! and wrong for everything else, and became visibly wrong once a phase stopped having to be
//! a full-viewport overlay to be recorded at all. The recorded rect answers both cases with
//! one rule: a curtain measures the whole viewport and lands exactly where the constant put
//! it, and an inline skeleton lands on the card it was standing in for.
//!
//! So this module exposes the renderings rather than the numbers. Nothing downstream restates
//! the arithmetic, which is what stops the CSS, the timer and the placement drifting apart.

use crate::model::{PageState, Rect};

/// Grace period after the phase finishes before the body scroll-lock is released, so the
/// lock cannot end mid-fade on a machine slower than the one captured.
const SETTLE_MARGIN_MS: u64 = 500;

/// One viewport's startup: its two phases, and the box the phase occupied.
#[derive(Clone, Copy)]
pub(super) struct Replay {
    delay_ms: u64,
    duration_ms: u64,
    rect: Rect,
}

impl Replay {
    pub(super) fn of(state: &PageState) -> Self {
        Self {
            delay_ms: state.startup_delay_ms,
            duration_ms: state.startup_duration_ms,
            rect: bounds(state),
        }
    }

    /// The custom properties the overlay reads, as an inline style body.
    ///
    /// The overlay's fill mode is `forwards`, which paints the base style during the delay
    /// rather than the first keyframe, so a real delay leaves the layer genuinely hidden and
    /// non-interactive until the moment the original page showed it.
    pub(super) fn style_variables(self) -> String {
        let Self {
            delay_ms,
            duration_ms,
            rect,
        } = self;
        format!(
            "\"--recreate-startup-delay\":\"{delay_ms}ms\",\
             \"--recreate-startup-duration\":\"{duration_ms}ms\",\
             \"--recreate-startup-x\":\"{x}px\",\"--recreate-startup-y\":\"{y}px\",\
             \"--recreate-startup-width\":\"{w}px\",\"--recreate-startup-height\":\"{h}px\"",
            x = length(rect.x),
            y = length(rect.y),
            w = length(rect.width),
            h = length(rect.height),
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

/// The box containing every root of the phase, in the viewport coordinates it was measured
/// in — which is the space a portalled `position: fixed` layer already uses, so there is no
/// conversion to get wrong.
fn bounds(state: &PageState) -> Rect {
    let mut edges: Option<(f64, f64, f64, f64)> = None;
    for rect in state
        .startup_nodes
        .iter()
        .filter(|node| node.parent.is_none())
        .map(|node| &node.rect)
    {
        let found = (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height);
        edges = Some(match edges {
            None => found,
            Some(held) => (
                held.0.min(found.0),
                held.1.min(found.1),
                held.2.max(found.2),
                held.3.max(found.3),
            ),
        });
    }
    let Some((left, top, right, bottom)) = edges else {
        return Rect::default();
    };
    Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

/// Two decimals is the precision the capture itself carries; trailing zeros only make the
/// generated style harder to read than the source it stands for.
fn length(value: f64) -> String {
    let text = format!("{value:.2}");
    match text.contains('.') {
        true => text.trim_end_matches('0').trim_end_matches('.').to_string(),
        false => text,
    }
}

#[cfg(test)]
#[path = "startup_replay_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "startup_placement_tests.rs"]
mod placement_tests;
