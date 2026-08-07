/// When the lifecycle recorder may stop watching the page.
///
/// The recorder used to run to a fixed 12-second horizon, so every capture of every page
/// paid twelve seconds whether or not anything was still moving. That became a ceiling, and
/// a one-second quiet period took its place — smaller, but still a constant standing in for
/// a measurement, and still charged to every page including one that never moves at all.
///
/// The page itself says how long a gap in its motion means the motion is over. The recorder
/// already sees every change it makes, so the longest gap it has watched the page recover
/// from is evidence: a page whose motion arrives in bursts 400ms apart has proven it can go
/// quiet for 400ms and resume, and a page that has never once paused and resumed has proven
/// nothing of the kind. Waiting longer than the longest gap already survived is therefore
/// the smallest wait that cannot cut short motion of the cadence this page has shown, and
/// it costs a page with no motion exactly one frame.
///
/// Running animations and pending loads are tracked separately as `busy`, because those are
/// direct evidence that something is still to come rather than an inference from history.
pub const SOURCE: &str = r#"
  const LIFECYCLE_CEILING_MS = 12000;
  const lifecycleSettled = (elapsed, sinceChange, busy, longestGap) =>
    elapsed >= LIFECYCLE_CEILING_MS || (!busy && sinceChange > longestGap);
"#;

#[cfg(test)]
#[path = "lifecycle_settle_tests.rs"]
mod tests;
