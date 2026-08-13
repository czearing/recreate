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
/// Evidence expires, though. An animation that repeats forever is running at every instant
/// the recorder can ever ask, so counting running animations charged every such page the
/// full ceiling and read the same motion over and over. A periodic process is described
/// completely by one period, so an animation stops being evidence once its delay and one
/// iteration have been watched; a finite animation reaches that point and ends there too.
/// A pending load has no period, so it holds on its own until it lands.
///
/// Waiting for that point only makes sense while the animation is still travelling towards
/// it. Local time advances at the playback rate, and only while the animation is running, so
/// a paused animation and one whose rate is zero are both stopped short of a point that will
/// never arrive — indistinguishable from each other and from a finished one in every way the
/// recorder cares about. Asking whether the clock is moving covers `finished` and `idle` as
/// the same case rather than as named exceptions, which is what keeps a third frozen state
/// from costing the full ceiling again.
///
/// Motion a stylesheet already declares is a further case of the same thing, reaching its
/// expiry before it starts. A `CSSAnimation` or a `CSSTransition` exists because a rule the
/// capture already reads called for it, so observing it re-derives what is written down;
/// only a script-built `Animation` describes motion no stylesheet records. That is what
/// keeps a page carrying a four-second authored loop from being watched for four seconds.
///
/// The same expiry has to reach the gap measurement, or a looping page never settles for a
/// different reason: it keeps moving, so the gap since the last change never grows. A change
/// on an element driven by an animation already watched through a full period is motion the
/// recorder has already described, not information it lacks, so `observedTargets` names the
/// elements whose movement no longer counts as news. Anything moving for any other reason —
/// a script ticking, content arriving — is not in that set and still holds the recorder.
///
/// `lifecycleDescribed` reads that same set straight off a document, because the settle scan
/// that decides a page has stopped moving needs it for the identical reason: a page carrying
/// a perpetual authored animation is never geometrically still, and waiting for stillness
/// there means waiting for the animation to happen to pause at a turning point.
pub const SOURCE: &str = r#"
  const LIFECYCLE_CEILING_MS = 12000;
  const animationObserved = ({ declared, playState, rate, delay, duration, localTime }) =>
    declared || playState !== 'running' || !rate ||
    !(duration > 0) || localTime >= delay + duration;
  const lifecycleLoading = root =>
    root.fonts.status !== 'loaded' ||
    Array.from(root.images).some(image => !image.complete && image.currentSrc);
  const lifecycleBusy = (animations, loading) =>
    loading || animations.some(animation => !animationObserved(animation));
  const observedTargets = animations =>
    new Set(animations.filter(animationObserved).map(animation => animation.target));
  const lifecycleTiming = animation => {
    const timing = animation.effect?.getComputedTiming?.() || {};
    return {
      target: animation.effect?.target,
      declared:
        (typeof CSSAnimation !== 'undefined' && animation instanceof CSSAnimation) ||
        (typeof CSSTransition !== 'undefined' && animation instanceof CSSTransition),
      playState: animation.playState,
      rate: animation.playbackRate ?? 1,
      delay: Number(timing.delay) || 0,
      duration: Number(timing.duration) || 0,
      localTime: Number(timing.localTime) || 0
    };
  };
  const lifecycleDescribed = root =>
    observedTargets(root.getAnimations({ subtree: true }).map(lifecycleTiming));
  const lifecycleSettled = (elapsed, sinceChange, busy, longestGap) =>
    elapsed >= LIFECYCLE_CEILING_MS || (!busy && sinceChange > longestGap);
  // A gap is silence the page came back from, so observing one takes two changes. The silence
  // before a page's first change is silence it never returned from, and counting it invents a
  // cadence out of the one interval that demonstrates none — on a page whose first edit lands
  // seconds after load, that invented gap is precisely what the recorder then waits out.
  const lifecycleGap = (widest, sinceLast, observed) =>
    observed ? Math.max(widest, sinceLast) : widest;
  const lifecycleLongestGap = times =>
    Array.from(times)
      .sort((left, right) => left - right)
      .reduce(
        (state, time, index) => ({
          widest: lifecycleGap(state.widest, time - state.previous, index > 0),
          previous: time
        }),
        { widest: 0, previous: 0 }
      ).widest;  const lifecycleAwaited = (soonestDue, start, changed) =>
    !changed && soonestDue <= start + LIFECYCLE_CEILING_MS;
"#;

#[cfg(test)]
#[path = "lifecycle_settle_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "lifecycle_busy_tests.rs"]
mod busy_tests;

#[cfg(test)]
#[path = "lifecycle_evidence_tests.rs"]
mod evidence_tests;
