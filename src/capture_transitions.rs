//! Makes a value that is still travelling arrive, so the style capture reads is a resting one.
//!
//! Capture reads a page through `getComputedStyle`, which reports the value in force at the
//! instant it is asked. A CSS transition makes that instant matter: for the length of the
//! transition the property is at some interpolated point between the value the page is
//! leaving and the value it is going to, and only the latter is the value the page rests at.
//! Reading mid-flight therefore records a value the author never wrote, and when the page is
//! leaving the property's initial value — the common case, because a transition that runs on
//! load starts from the initial value — what gets recorded is indistinguishable from an
//! unauthored default and is pruned as one. The declaration is then simply gone.
//!
//! Deciding this by waiting cannot work. The wait costs whatever duration the page chose,
//! charged once per viewport, and any wait must be bounded, so the bound is reached exactly
//! on the pages with the longest transitions and those are captured mid-flight anyway. A
//! guarantee that lapses on its worst input is not one.
//!
//! Seeking is exact and costs nothing. A `CSSTransition` is an `Animation`, so it carries
//! `finish()`, which moves it to the end of its active interval at once; the page then holds
//! the style it was going to hold a moment later regardless. Nothing is invented, and the
//! motion is not lost, because the lifecycle recorder samples and stores it separately.
//!
//! The reading a capture takes is wrapped rather than merely preceded, because the baseline
//! measurement in the middle of it is the page's largest source of transitions: it reverts
//! every element to the user-agent origin and puts its style attribute back, and both are
//! style changes that an element declaring a transition answers by starting one. A reading
//! taken before that pass is stale by the end of it, and a baseline read during it is taken
//! while a transition is applying over the reverted value, which records a difference that is
//! really just the passage of time. So the pass runs with transitions suspended — declared
//! away by one stylesheet, which also cancels anything already running and leaves it at the
//! value it was going to — and what the pass provoked cannot exist, because the change was
//! committed while there was nothing to provoke. Afterwards the page keeps every transition
//! it declares; it simply has none outstanding.
//!
//! The first-paint reading is wrapped in the other policy, and must be: it is a snapshot of
//! one moment, every entry transition the page declares is in flight during it, and ending
//! those there reads a later page than the one requested and leaves the page holding values
//! that stop the motion being provoked again, so it is lost to every later reader too.
//!
//! Only transitions. A transition interpolates the base style towards the after-change
//! style, so its end value *is* the resting computed value. An animation applies in a higher
//! cascade origin that overrides the base style, so its end value is not what the element
//! rests at, and an animation may repeat forever, which makes "let it reach the end" a thing
//! that never happens. That is a difference in what the two kinds of motion mean, not a list
//! of cases, which is why it is expressed as the platform's own type rather than as names.
//!
//! Nothing about the motion is lost by ending it. The lifecycle recorder samples motion as it
//! runs and the first-paint reading holds the frame the page began on, so how the page arrived
//! is recorded by the stages whose subject that is, and this one is left free to record only
//! where it arrived.
pub const SOURCE: &str = r#"
  const transitional = animation =>
    typeof CSSTransition !== 'undefined' && animation instanceof CSSTransition;
  const arriveTransitions = root => {
    for (const animation of root.getAnimations({ subtree: true })) {
      if (!transitional(animation)) continue;
      // An animation whose end time is unresolved cannot be sought to it, and says so by
      // throwing. That is a transition still without a resting value, so there is nothing
      // to bring forward and skipping it is the whole response.
      try { animation.finish(); } catch (unresolved) {}
    }
  };
  // Declaring the transitions away rather than pausing them, because a paused transition is
  // still applying its interpolated value over the one being measured. Removing the sheet
  // provokes nothing: every change made under it is already the page's current value.
  const restingRead = read => {
    const suspended = document.createElement('style');
    suspended.textContent = '*,*::before,*::after{transition-property:none !important}';
    document.head.appendChild(suspended);
    try { read(); } finally { suspended.remove(); }
    // A stylesheet reaches the document tree only. A shadow tree has its own cascade, so
    // whatever is still running inside one is brought to its end by the rule directly.
    arriveTransitions(document);
  };
  const movingRead = read => read();
"#;
