//! Width-interval evidence.
//!
//! A constraint on a responsive page holds over a range of widths, not at one
//! width. A comparison recorded at a single viewport is therefore blind to
//! every defect that lives between breakpoints, however many properties it
//! measures at that one width.
//!
//! This module samples geometry across the width axis and turns the samples
//! into inclusive width intervals. Every rule here is deliberately pure so the
//! interval a report prints can be recomputed from recorded evidence alone,
//! which is what makes repeated runs produce identical report JSON.

use crate::model::{SweepNode, SweepProbe};
use std::collections::BTreeSet;

/// Narrower than this no longer describes a page anyone renders, and wider than
/// this is a desktop that no media query in practice distinguishes.
pub const MINIMUM_WIDTH: u32 = 320;
pub const MAXIMUM_WIDTH: u32 = 1920;
/// A page with many breakpoints must still cost a bounded number of reflows.
pub const MAXIMUM_PROBES: usize = 32;
/// Refinement only ever runs on a page that already failed, but a failing page
/// must not be allowed to cost more than a passing one by an unbounded amount.
pub const MAXIMUM_REFINEMENTS: usize = 8;

/// The pixel integers a page's own width-constrained conditions declare.
///
/// A device-name list would encode the fixture author's guesses instead of the
/// page's, and would miss every breakpoint the page actually has.
pub fn boundaries(bands: &[String]) -> Vec<u32> {
    let mut values = BTreeSet::new();
    for band in bands {
        let bytes = band.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if !bytes[index].is_ascii_digit() {
                index += 1;
                continue;
            }
            let start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            // Only a length stated in pixels is a width in the units a probe
            // can be set to; `37.5em` depends on a font size the sweep has not
            // measured, so it is not turned into a guessed pixel boundary.
            if band[index..].starts_with("px")
                && let Ok(value) = band[start..index].parse::<u32>()
            {
                values.insert(value);
            }
        }
    }
    values.into_iter().collect()
}

/// Every width the source is sampled at.
///
/// Each boundary is probed at one pixel below, at, and one pixel above, so a
/// defect seated exactly on a breakpoint cannot hide between samples, and the
/// midpoint of each consecutive pair is probed so a defect in the middle of a
/// band cannot either. The session width is always included so the swept
/// evidence and the recorded state describe the same page.
pub fn probe_widths(bands: &[String], session_width: u32) -> Vec<u32> {
    let boundaries = boundaries(bands);
    if boundaries.is_empty() {
        return Vec::new();
    }
    let mut widths = BTreeSet::new();
    for boundary in &boundaries {
        for offset in [-1i64, 0, 1] {
            widths.insert(clamp(*boundary as i64 + offset));
        }
    }
    for pair in boundaries.windows(2) {
        widths.insert(clamp(pair[0] as i64 + (pair[1] as i64 - pair[0] as i64) / 2));
    }
    widths.insert(clamp(session_width as i64));
    thin(widths, clamp(session_width as i64))
}

fn clamp(value: i64) -> u32 {
    value.clamp(MINIMUM_WIDTH as i64, MAXIMUM_WIDTH as i64) as u32
}

/// Keeps the sample count bounded without ever dropping the session width, and
/// without depending on iteration order, so the same bands always yield the
/// same list.
fn thin(widths: BTreeSet<u32>, session_width: u32) -> Vec<u32> {
    if widths.len() <= MAXIMUM_PROBES {
        return widths.into_iter().collect();
    }
    let ordered = widths.into_iter().collect::<Vec<_>>();
    let stride = ordered.len().div_ceil(MAXIMUM_PROBES).max(2);
    let mut kept = ordered
        .iter()
        .step_by(stride)
        .copied()
        .collect::<BTreeSet<_>>();
    kept.insert(session_width);
    while kept.len() > MAXIMUM_PROBES {
        let last = *kept.iter().next_back().expect("non-empty");
        let drop = if last == session_width {
            *kept.iter().next().expect("non-empty")
        } else {
            last
        };
        kept.remove(&drop);
    }
    kept.into_iter().collect()
}

/// The source's value for a node at a width it was never sampled at.
///
/// The source is recorded once per comparison, so an intermediate width has no
/// source evidence of its own. Where the source is identical at the two
/// sampled widths that enclose it, the constraint is treated as constant across
/// that interval; where the source itself changes, nothing is claimed, and the
/// candidate axis is not refined there.
pub fn source_at<'a>(
    probes: &'a [SweepProbe],
    width: u32,
    target: &str,
) -> Option<Option<&'a SweepNode>> {
    if let Some(probe) = probes.iter().find(|probe| probe.width == width) {
        return Some(probe.nodes.get(target));
    }
    let below = probes.iter().filter(|probe| probe.width < width).next_back()?;
    let above = probes.iter().find(|probe| probe.width > width)?;
    let left = below.nodes.get(target);
    let right = above.nodes.get(target);
    (left == right).then_some(left)
}

/// Whether the candidate departs from the source at one width, for one node.
fn differs(source: &[SweepProbe], candidate: &SweepProbe, target: &str) -> Option<bool> {
    let expected = source_at(source, candidate.width, target)?;
    Some(expected != candidate.nodes.get(target))
}

/// The nodes worth locating an interval for, in a stable order.
pub fn diverging_targets(source: &[SweepProbe], candidate: &[SweepProbe]) -> Vec<String> {
    let mut targets = BTreeSet::new();
    for probe in candidate {
        for target in probe.nodes.keys() {
            targets.insert(target.clone());
        }
    }
    for probe in source {
        for target in probe.nodes.keys() {
            targets.insert(target.clone());
        }
    }
    targets
        .into_iter()
        .filter(|target| {
            candidate
                .iter()
                .any(|probe| differs(source, probe, target).unwrap_or(false))
        })
        .collect()
}

/// The inclusive width interval a node differs over.
///
/// The interval is simply the contiguous run of differing widths containing the
/// lowest differing width, taken over every width actually probed. Refinement
/// therefore needs no agreement with this function: an extra probe that differs
/// extends the run, and an extra probe that matches bounds it.
pub fn interval(source: &[SweepProbe], candidate: &[SweepProbe], target: &str) -> Option<(u32, u32)> {
    let samples = candidate
        .iter()
        .filter_map(|probe| Some((probe.width, differs(source, probe, target)?)))
        .collect::<Vec<_>>();
    let seed = samples.iter().position(|(_, differs)| *differs)?;
    let mut lo = seed;
    while lo > 0 && samples[lo - 1].1 {
        lo -= 1;
    }
    let mut hi = seed;
    while hi + 1 < samples.len() && samples[hi + 1].1 {
        hi += 1;
    }
    Some((samples[lo].0, samples[hi].0))
}

/// Drives the extra candidate probes that pin an interval endpoint to the exact
/// pixel.
///
/// "Differs at width W" is false, then true, then false across the axis, because
/// a band diverges inside and matches outside. Bisecting that whole axis is
/// bisecting a non-monotone predicate, and it can terminate on different
/// endpoints on different runs. So each edge is searched only inside the
/// half-open interval between an observed differing width and its nearest
/// observed matching neighbour, where the predicate is monotone and the answer
/// is unique.
#[derive(Debug)]
pub struct Refiner {
    edges: Vec<Edge>,
    budget: usize,
}

#[derive(Debug)]
struct Edge {
    target: String,
    /// Known to match. Never probed again.
    outside: u32,
    /// Known to differ.
    inside: u32,
    /// Whether `outside` sits below `inside`.
    below: bool,
}

impl Edge {
    fn next(&self) -> Option<u32> {
        let (low, high) = if self.below {
            (self.outside, self.inside)
        } else {
            (self.inside, self.outside)
        };
        (high - low > 1).then(|| low + (high - low) / 2)
    }

    fn accept(&mut self, width: u32, differs: bool) {
        if differs {
            self.inside = width;
        } else {
            self.outside = width;
        }
    }
}

impl Refiner {
    /// Plans refinement from the probes already taken. Returns nothing at all
    /// when the candidate matches the source everywhere, so a passing
    /// comparison pays for no refinement.
    pub fn plan(source: &[SweepProbe], candidate: &[SweepProbe]) -> Self {
        let mut edges = Vec::new();
        for target in diverging_targets(source, candidate) {
            let Some((lo, hi)) = interval(source, candidate, &target) else {
                continue;
            };
            let below = candidate
                .iter()
                .map(|probe| probe.width)
                .filter(|width| *width < lo)
                .next_back();
            let above = candidate
                .iter()
                .map(|probe| probe.width)
                .find(|width| *width > hi);
            if let Some(outside) = below
                && lo - outside > 1
                && refinable(source, outside, lo, &target)
            {
                edges.push(Edge {
                    target: target.clone(),
                    outside,
                    inside: lo,
                    below: true,
                });
            }
            if let Some(outside) = above
                && outside - hi > 1
                && refinable(source, hi, outside, &target)
            {
                edges.push(Edge {
                    target: target.clone(),
                    outside,
                    inside: hi,
                    below: false,
                });
            }
        }
        Self {
            edges,
            budget: MAXIMUM_REFINEMENTS,
        }
    }

    /// The next width to probe on the candidate, or nothing when every edge is
    /// pinned or the budget is spent.
    pub fn next_width(&self) -> Option<u32> {
        if self.budget == 0 {
            return None;
        }
        self.edges.iter().find_map(Edge::next)
    }

    /// Records a probe the driver took. One probe carries every node, so each
    /// edge takes its verdict from its own node.
    pub fn accept(&mut self, source: &[SweepProbe], probe: &SweepProbe) {
        self.budget = self.budget.saturating_sub(1);
        for edge in &mut self.edges {
            let Some(differs) = differs(source, probe, &edge.target) else {
                continue;
            };
            if edge.next() == Some(probe.width) {
                edge.accept(probe.width, differs);
            }
        }
    }
}

/// Whether the source is constant across a gap, which is the precondition for
/// bisecting the candidate axis against it.
fn refinable(source: &[SweepProbe], low: u32, high: u32, target: &str) -> bool {
    let left = source_at(source, low, target);
    let right = source_at(source, high, target);
    matches!((left, right), (Some(left), Some(right)) if left == right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn bands(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    fn probe(width: u32, x: f64) -> SweepProbe {
        SweepProbe {
            width,
            nodes: BTreeMap::from([(
                "mark".into(),
                SweepNode {
                    visible: true,
                    x,
                    y: 0.0,
                    width: 200.0,
                    height: 20.0,
                    transform: "none".into(),
                },
            )]),
        }
    }

    #[test]
    fn probe_widths_come_from_the_pages_own_declared_boundaries() {
        let widths = probe_widths(
            &bands(&[
                "(min-width:450px)and(max-width:479px)",
                "(min-width:480px)and(max-width:499px)",
                "(min-width:600px)and(max-width:600px)",
                "(min-width:601px)and(max-width:661px)",
                "(min-width:662px)and(max-width:900px)",
            ]),
            1440,
        );
        // Every boundary is probed at minus one, at, and plus one.
        for boundary in [450, 479, 480, 499, 600, 601, 661, 662, 900] {
            for width in [boundary - 1, boundary, boundary + 1] {
                assert!(widths.contains(&width), "missing probe {width}");
            }
        }
        // The midpoint of a wide band, so a defect between boundaries is seen.
        assert!(widths.contains(&781));
        // The session width, so the sweep and the recorded state agree.
        assert!(widths.contains(&1440));
        assert!(widths.len() <= MAXIMUM_PROBES);
        assert_eq!(widths, {
            let mut sorted = widths.clone();
            sorted.sort_unstable();
            sorted.dedup();
            sorted
        });
    }

    #[test]
    fn a_page_without_width_conditional_css_is_never_swept() {
        assert!(probe_widths(&[], 1440).is_empty());
        assert!(probe_widths(&bands(&["(min-width:37.5em)"]), 1440).is_empty());
    }

    #[test]
    fn probe_widths_stay_inside_the_renderable_range_and_the_cap() {
        let many = (1..40)
            .map(|index| format!("(min-width:{}px)", index * 40))
            .collect::<Vec<_>>();
        let widths = probe_widths(&many, 1440);
        assert!(widths.len() <= MAXIMUM_PROBES);
        assert!(widths.contains(&1440));
        assert!(widths.iter().all(|width| (MINIMUM_WIDTH..=MAXIMUM_WIDTH).contains(width)));
        assert_eq!(widths, probe_widths(&many, 1440));
    }

    #[test]
    fn an_interval_is_the_contiguous_run_of_differing_widths() {
        let source = vec![
            probe(449, 24.0),
            probe(450, 24.0),
            probe(464, 24.0),
            probe(479, 24.0),
            probe(480, 24.0),
        ];
        let candidate = vec![
            probe(449, 24.0),
            probe(450, 159.0),
            probe(464, 159.0),
            probe(479, 159.0),
            probe(480, 24.0),
        ];
        assert_eq!(interval(&source, &candidate, "mark"), Some((450, 479)));
    }

    #[test]
    fn a_one_pixel_band_reports_the_same_width_twice() {
        let source = vec![probe(599, 24.0), probe(600, 24.0), probe(601, 24.0)];
        let candidate = vec![probe(599, 24.0), probe(600, 48.0), probe(601, 24.0)];
        assert_eq!(interval(&source, &candidate, "mark"), Some((600, 600)));
    }

    #[test]
    fn a_matching_page_yields_no_interval_and_no_refinement() {
        let source = vec![probe(599, 24.0), probe(600, 24.0), probe(601, 24.0)];
        assert_eq!(interval(&source, &source, "mark"), None);
        assert!(diverging_targets(&source, &source).is_empty());
        assert!(Refiner::plan(&source, &source).next_width().is_none());
    }

    /// The whole width axis is false-true-false, so bisecting it could settle on
    /// different endpoints per run. Refinement searches only the monotone half
    /// intervals either side of an observed differing probe, so the same input
    /// must always produce the same endpoints.
    #[test]
    fn refinement_is_deterministic_on_a_false_true_false_predicate() {
        let source = (0..=10)
            .map(|index| probe(400 + index * 20, 24.0))
            .collect::<Vec<_>>();
        let diverges = |width: u32| (430..=530).contains(&width);
        let run = || {
            let mut candidate = source
                .iter()
                .map(|value| probe(value.width, if diverges(value.width) { 99.0 } else { 24.0 }))
                .collect::<Vec<_>>();
            let mut refiner = Refiner::plan(&source, &candidate);
            let mut visited = Vec::new();
            while let Some(width) = refiner.next_width() {
                let taken = probe(width, if diverges(width) { 99.0 } else { 24.0 });
                refiner.accept(&source, &taken);
                visited.push(width);
                candidate.push(taken);
                candidate.sort_by_key(|value| value.width);
            }
            (visited, interval(&source, &candidate, "mark"))
        };
        let first = run();
        assert_eq!(first, run());
        assert_eq!(first, run());
        assert_eq!(first.1, Some((430, 530)));
        assert!(first.0.len() <= MAXIMUM_REFINEMENTS);
    }

    #[test]
    fn an_unprobed_width_is_only_claimed_where_the_source_is_constant() {
        let probes = vec![probe(400, 24.0), probe(500, 24.0), probe(600, 90.0)];
        assert!(source_at(&probes, 450, "mark").is_some());
        // The source itself moves between 500 and 600, so nothing is claimed of
        // the widths in between and the candidate axis is not refined there.
        assert!(source_at(&probes, 550, "mark").is_none());
        assert!(!refinable(&probes, 500, 600, "mark"));
        assert!(refinable(&probes, 400, 500, "mark"));
    }
}
