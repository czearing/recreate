//! Holds motion out of a reading, so the style capture records is the style the page rests at.
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
//! One rule decides both kinds of motion, and it is about where the motion's endpoint comes
//! from. A transition only delays the after-change style, which is the value the cascade
//! already produced, so seeking it to its end lands on the value the page rests at and
//! invents nothing. An animation declares its own endpoints as keyframes in an origin above
//! the cascade, so no frame it is passing through is that value, and a property an animation
//! drives but the author never declared rests at a value that appears in no keyframe at all.
//! Motion whose end the cascade produced is sought; motion that declares its own is held out
//! of the read. That also answers, rather than merely accommodates, why an animation may not
//! be finished: the frame it would land on is the wrong value, and an endless one has no end
//! to reach.
//!
//! An animation is not held out of the read entirely, though, because one of its frames does
//! outlive it. A fill keeps the effect applying after the active interval ends, indefinitely,
//! and there is no later moment at which the element reverts — so that value is not a frame
//! the clock caught, it is where the page rests, and every closed Drawer in the shipped
//! bundle rests on one with nothing in its own declarations saying so. So each effect is
//! offered the place its animation comes to rest — the end it is travelling towards while it
//! is running, and wherever it already stands when it is not — and the engine decides what
//! applies there. A fill applies; a `none` fill applies nothing; an endless animation has no
//! resting time to be offered. Nothing here reads what was written, so `forwards`, `both`, a
//! reversed direction and a script-started animation all travel the same path.
//!
//! Held out by detaching the effect, because that is the one handle every animation has. A
//! declaration would reach only what the CSS owns, leaving anything a script started still
//! applying its frame, and declaring `animation-name` away would delete the very longhands
//! the recreation needs to animate at all. Detaching keeps the animation, its timeline
//! position and its play state, so the page is left moving exactly as it was found; the
//! alternatives do not, since cancelling discards all three and pausing goes on applying the
//! frame — the same objection this file already makes to a paused transition. A fill is kept
//! by moving the same effect, not the animation: an effect belongs to one animation at a
//! time, so a detached one can be presented from a scratch animation held at its end while
//! the page's own is left exactly as detaching already left it.
//!
//! The hold spans the whole reading rather than one pass, because every value a capture
//! records is read from the page and an animation is applying throughout. It is released
//! before the motion itself is recorded, so the reader whose subject is how the page moves
//! still sees every animation the page has.
//!
//! Nothing about the motion is lost by ending or suspending it. The lifecycle recorder
//! samples motion as it runs and the first-paint reading holds the frame the page began on,
//! so how the page arrived is recorded by the stages whose subject that is, and this one is
//! left free to record only where it arrived.
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
  // Detaching the effect leaves the animation itself untouched, so what is put back is the
  // same effect on the same animation at the same point on its timeline.
  //
  // A detached effect applies nothing, which is right for a frame and wrong for a fill. So it
  // is offered the place it comes to rest: a scratch animation on the same timeline, seeked to
  // that time. Whether anything is applied there is then not a question anyone has to ask — the
  // engine answers it by applying a fill and applying nothing otherwise, so `forwards`, `both`,
  // `backwards`, `none`, a reversed direction and a script-started animation all arrive at the
  // right value without a clause each. An effect with no finite resting time has no such moment
  // to be offered, which is the whole of the endless case.
  //
  // The scratch animations are relevant animations over the page's own elements, so the page
  // reports them like any other and the second sweep would hold them in turn. Knowing them is
  // what keeps one effect under one holder, since two releases claiming the same effect leave
  // it wherever the last one put it.
  const restingHolders = new WeakSet();
  // Where the animation stops. It is only going anywhere while it is running, and which way it
  // is going is the sign of its rate; a pause, a zero rate and a finish are all the same
  // statement, which is that it is already there. Reading the end time of an animation that
  // will never reach it would record a frame the page never shows, which is the defect this
  // policy exists to prevent rather than a second version of it.
  const restingTime = animation => {
    const rate = animation.playState === 'running' ? animation.playbackRate : 0;
    if (rate > 0) return animation.effect.getComputedTiming().endTime;
    if (rate < 0) return 0;
    return animation.currentTime;
  };
  // The engine does not report an animation with nothing to apply, so a hold never meets one and
  // needs no guard against it. Measured rather than assumed: an animation whose effect has been
  // set to null leaves `getAnimations()` at once, as does one that has finished without a fill.
  const holdAtRest = (animation, effect, at) => {
    if (!Number.isFinite(at)) return;
    const holder = new Animation(effect, animation.timeline);
    restingHolders.add(holder);
    holder.currentTime = at;
  };
  const suspendAnimations = root => {
    const suspended = [];
    for (const animation of root.getAnimations({ subtree: true })) {
      if (transitional(animation) || restingHolders.has(animation)) continue;
      const effect = animation.effect;
      const at = restingTime(animation);
      animation.effect = null;
      holdAtRest(animation, effect, at);
      suspended.push([animation, effect]);
    }
    return () => {
      // Putting the effect back takes it off whatever was presenting it, because an effect
      // belongs to one animation at a time. So the holder needs no dismissal of its own; it is
      // left with nothing to apply and the page stops reporting it.
      for (const [animation, effect] of suspended) animation.effect = effect;
      suspended.length = 0;
    };
  };
  // Declaring the transitions away rather than pausing them, because a paused transition is
  // still applying its interpolated value over the one being measured. Removing the rules
  // provokes nothing: every change made under them is already the page's current value.
  const restingRead = read => {
    // Twice, because the measurement in the middle of the read is the page's largest source of
    // motion for the same reason it is its largest source of transitions: reverting an element
    // and putting its style attribute back removes and restores `animation-name`, which ends
    // every animation held out here and starts a fresh one in its place. Holding out what was
    // running before the read leaves the measurement itself resting; holding out what the
    // measurement started leaves every reading taken after it resting too.
    const releases = [suspendAnimations(document)];
    underRules('*,*::before,*::after{transition-property:none !important}', read);
    releases.push(suspendAnimations(document));
    // Suspending a transition holds its interpolated value out of the read; it does not give
    // the transition a resting value. Every later stage reads a page that has one, so the
    // motion still in flight is brought to its end once the read is over.
    arriveTransitions(document);
    return () => { for (const release of releases) release(); };
  };
  const movingRead = read => {
    read();
    return () => {};
  };
"#;
