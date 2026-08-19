//! The page the motion tests read, shared so that both halves of the one rule — motion sought
//! to its end and motion held out of the read — are checked against the same page rather than
//! against two doubles that could drift apart and agree with each other by accident.
//!
//! The double is shaped like the platform rather than like the tests: an effect declares where
//! the motion goes and reports, for wherever its host has placed it, whether it applies anything
//! there. That is the only shape in which "the value a finished fill holds" and "the frame a
//! running animation is passing through" are different facts, and the rule under test is exactly
//! the one that tells them apart.

use crate::node_eval;
pub const DOUBLE: &str = r#"
globalThis.style = {};
globalThis.animations = [];
const timeline = { name: 'document' };
// One duration for every effect in the double, because nothing under test reads a duration; what
// is read is whether a local time is past the end and whether the effect still applies there.
const ACTIVE_END = 200;
// The endpoint travels with the effect, as it does on the platform: an effect is what declares
// where the motion is going, so an animation whose effect has been detached has no end left to be
// sought to, and no value left to apply. That is why holding motion out and bringing it to its
// end are not interchangeable, and it is the only place the double can express it.
class KeyframeEffect {
  constructor(property, options) {
    this.property = property;
    this.from = options.from;
    this.to = options.to;
    this.frame = options.frame;
    this.endless = options.endless === true;
    this.fills = options.fills === true;
    this.localTime = null;
  }
  getComputedTiming() {
    // The engine reports no end time at all for an animation that never ends, rather than an
    // infinite one, so the double reports the same and the finiteness question is asked of the
    // same answer production will see.
    const endTime = this.endless ? null : ACTIVE_END;
    const ended = endTime !== null && this.localTime !== null && this.localTime >= endTime;
    const before = this.localTime !== null && this.localTime < 0;
    const applies = this.localTime !== null && ((!ended && !before) || this.fills);
    return { endTime, progress: applies ? (ended ? 1 : before ? 0 : 0.5) : null };
  }
  // What a reader sees the element at: the frame while the motion is passing through, the value
  // at whichever end a fill holds it against, and nothing at all once it has let go.
  get applied() {
    const { endTime, progress } = this.getComputedTiming();
    if (progress === null) return undefined;
    if (endTime !== null && this.localTime >= endTime) return this.to;
    if (this.localTime <= 0) return this.from === undefined ? this.frame : this.from;
    return this.frame;
  }
}
class Animation {
  constructor(effect, animationTimeline) {
    this.timeline = animationTimeline;
    this.playState = 'idle';
    this.playbackRate = 1;
    this.held = null;
    this.attached = null;
    this.effect = effect || null;
    globalThis.animations.push(this);
  }
  get effect() { return this.attached; }
  set effect(next) {
    if (this.attached) this.attached.localTime = null;
    // An effect belongs to at most one animation, so handing it to another takes it away from
    // the one that had it. Without that, two animations can both believe they hold it and a
    // release that runs in the wrong order looks like it worked.
    if (next && next.host && next.host !== this) next.host.attached = null;
    this.attached = next || null;
    if (this.attached) {
      this.attached.host = this;
      this.attached.localTime = this.held;
    }
  }
  get currentTime() { return this.held; }
  // The engine refuses a current time that is not a real number, measured rather than assumed:
  // seeking a scratch animation to the end time of an endless effect throws `TypeError`, and a
  // throw here has no catch above it, so the whole capture would end with no artifact.
  set currentTime(time) {
    if (typeof time !== 'number' || !isFinite(time)) {
      throw new TypeError('current time must be a finite number');
    }
    this.held = time;
    if (this.playState === 'idle') this.playState = 'paused';
    if (this.attached) this.attached.localTime = time;
  }
  finish() {
    if (!this.attached) throw new Error('no effect to seek');
    if (this.attached.endless) throw new Error('unresolved end time');
    globalThis.style[this.attached.property] = this.attached.to;
    this.playState = 'finished';
    this.effect = null;
  }
}
class CSSTransition extends Animation {}
class CSSAnimation extends Animation {}
// An animation with no effect applies nothing and is not a relevant animation, so the platform
// stops reporting it. Modelling that is what lets a test say an animation was put back rather
// than merely that some object still exists.
const relevant = () => globalThis.animations.filter(animation => animation.effect);
const started = (Kind, name, property, options) => {
  const animation = new Kind(new KeyframeEffect(property, options), timeline);
  animation.name = name;
  animation.currentTime = options.at === undefined ? ACTIVE_END / 2 : options.at;
  animation.playState = options.playState || 'running';
  if (options.rate !== undefined) animation.playbackRate = options.rate;
  return animation;
};
const transition = (name, property, to, options) =>
  started(CSSTransition, name, property, Object.assign({ to }, options));
const animate = (name, property, options) => started(CSSAnimation, name, property, options);
const scripted = (name, property, options) => started(Animation, name, property, options);
const computed = property => {
  for (const animation of relevant()) {
    const effect = animation.effect;
    if (effect.property === property && effect.applied !== undefined) return effect.applied;
  }
  return globalThis.style[property];
};
const root = { getAnimations: options => (options && options.subtree ? relevant() : []) };
const names = () => relevant().map(animation => animation.name);
class CSSStyleSheet {
  replaceSync(text) { this.text = text; }
}
globalThis.CSSStyleSheet = CSSStyleSheet;
const document = {
  querySelectorAll: () => [],
  styleSheets: [],
  adoptedStyleSheets: [],
  getAnimations: options => root.getAnimations(options)
};
// What the page is being read under, named as the rule texts in force rather than as the
// carrier that delivers them, so the assertions survive a change of carrier.
Object.defineProperty(globalThis, 'sheets', {
  get: () => document.adoptedStyleSheets.map(sheet => sheet.text)
});
"#;

pub fn evaluate(body: &str, expression: &str) -> serde_json::Value {
    node_eval::evaluate(
        &format!(
            "{DOUBLE}\n{}\n{}\n{body}",
            crate::scoped_rules::SOURCE,
            crate::capture_motion::SOURCE
        ),
        expression,
    )
}
