/// When the lifecycle recorder may stop watching the page.
///
/// The recorder used to run to a fixed 12-second horizon, so every capture of every page
/// paid twelve seconds whether or not anything was still moving — a constant standing in
/// for a measurement of the page. The horizon is now only a ceiling. The window closes as
/// soon as nothing is left that could still change the page: no animation is running,
/// nothing is still loading, and the last recorded change is older than the quiet period.
/// Any change restarts the quiet period, so chained and delayed motion keeps the recorder
/// alive for exactly as long as the page keeps moving.
pub const SOURCE: &str = r#"
  const LIFECYCLE_CEILING_MS = 12000;
  const LIFECYCLE_QUIET_MS = 1000;
  const lifecycleSettled = (elapsed, sinceChange, busy) =>
    elapsed >= LIFECYCLE_CEILING_MS || (!busy && sinceChange >= LIFECYCLE_QUIET_MS);
"#;

#[cfg(test)]
#[path = "lifecycle_settle_tests.rs"]
mod tests;
