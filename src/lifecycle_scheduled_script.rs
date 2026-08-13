/// What a purely script-driven page says about work it has not done yet.
///
/// The recorder settles when nothing is running and the page has been quiet for longer than
/// the longest gap it has already recovered from. Before the page's first change that rule
/// reads "nothing has changed yet" as "nothing will change": the longest observed gap is
/// zero, one frame of quiet exceeds it, and the window closes on the first frame. A page
/// whose motion arrives from a timer therefore has its entire progression missed, because
/// the recorder gave up before the first edit ran.
///
/// A scheduled timer is the missing evidence, and it is evidence of the same kind the
/// recorder already trusts: a request in flight is not an inference from history, it is the
/// page stating directly that more is to come. A timer states the same thing and states it
/// more precisely, because it also says when.
///
/// Evidence expires as it is consumed. A timeout stops counting once it fires or once it is
/// cancelled. A repeating interval counts only until its first tick, for the same reason a
/// repeating animation stops counting after one period: one period describes a periodic
/// process completely, and waiting for the second tick learns nothing the first did not
/// already show.
///
/// Two bounds apply, and they answer different questions. Work scheduled to happen after the
/// recorder's own ceiling cannot produce anything the recorder will ever see, so it is not
/// evidence for the recorder at all. Work scheduled before it stops being evidence the moment
/// the page makes its first edit: the blind spot this schedule exists to cover is the silence
/// before that edit, and once the page has moved it has demonstrated the cadence the
/// observed-gap rule measures, which is the stronger evidence of the two. Without that second
/// bound a routine up-front heartbeat or deferred banner held the recorder to its ceiling on
/// every production page, while a fixture that schedules nothing never noticed.
///
/// Evidence must also be non-renewable, and this is the clause the whole rule turns on. The
/// recorder's blind spot is one specific interval: the silence before the page's first edit,
/// where the longest observed gap is still zero. A page's up-front schedule is exactly the
/// evidence that covers it. Once that schedule starts running, the recorder is watching a
/// page that has begun, and the observed-gap rule it already trusts takes over — so work
/// scheduled after the first timer has fired adds nothing and is not admitted.
///
/// Refusing renewals is what makes the rule terminate. A poll loop has one timer outstanding
/// at every instant and never a last one, so admitting its renewals would hold the recorder
/// open for as long as the loop ran. Measured on a page with no script of its own, a browser
/// extension's poll loop kept the recorder busy for 300 of 301 frames and cost 5.6s per
/// capture. The loop reschedules from a promise continuation rather than from inside the
/// timer callback, so nothing about the shape of the call can be relied on; only the fact
/// that the schedule had already started can. It is the same principle already applied to a
/// repeating interval, which is itself a chain the browser renews.
pub const SOURCE: &str = r#"
  const trackScheduled = scope => {
    const due = new Map();
    const consume = handle => { due.delete(handle); };
    const original = scope.setTimeout.bind(scope);
    let running = false;
    const wrap = schedule => (handler, delay, ...rest) => {
      if (typeof handler !== 'function') return schedule(handler, delay, ...rest);
      const renewal = running;
      const at = scope.performance.now() + (Number(delay) || 0);
      let handle;
      handle = schedule((...args) => {
        consume(handle);
        running = true;
        return handler(...args);
      }, delay, ...rest);
      if (!renewal) due.set(handle, at);
      return handle;
    };
    scope.setTimeout = wrap(original);
    scope.setInterval = wrap(scope.setInterval.bind(scope));
    const originalClearTimeout = scope.clearTimeout.bind(scope);
    const originalClearInterval = scope.clearInterval.bind(scope);
    scope.clearTimeout = handle => { consume(handle); return originalClearTimeout(handle); };
    scope.clearInterval = handle => { consume(handle); return originalClearInterval(handle); };
    scope.__recreateTimeout = original;
    return () => Math.min(Infinity, ...due.values());
  };
"#;

/// The scheduler every capture instrument must use, named once so no call site can drift.
///
/// The harness polls the page from inside the page, and each poll is scheduled from the
/// harness's own async loop rather than from a timer callback, so it is first-generation
/// work by every rule above and would be counted as the page's. The instrument would then
/// hold the recorder open until the instrument stopped — which it only does once the
/// recorder closes. An instrument must not appear in its own measurement, so it reaches the
/// scheduler the tracker set aside before wrapping. The fallback covers instruments running
/// before the tracker is installed, and pages where it never is.
pub const INSTRUMENT_TIMEOUT: &str = "(window.__recreateTimeout || setTimeout)";

#[cfg(test)]
#[path = "lifecycle_scheduled_tests.rs"]
mod tests;
