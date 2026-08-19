//! The page the motion tests read, shared so that both halves of the one rule — motion sought
//! to its end and motion held out of the read — are checked against the same page rather than
//! against two doubles that could drift apart and agree with each other by accident.

use crate::node_eval;
pub const DOUBLE: &str = r#"
globalThis.style = {};
globalThis.running = [];
class Animation {
  constructor(name, property, to, endless, applied) {
    this.name = name; this.property = property; this.to = to; this.endless = endless;
    this.playState = 'running';
    this.effect = applied === undefined ? null : { property, frame: applied };
    globalThis.running.push(this);
  }
  finish() {
    if (this.endless) throw new Error('unresolved end time');
    globalThis.style[this.property] = this.to;
    globalThis.running = globalThis.running.filter(other => other !== this);
  }
}
class CSSTransition extends Animation {}
class CSSAnimation extends Animation {}
const computed = property => {
  for (const animation of globalThis.running) {
    if (animation.effect && animation.effect.property === property) return animation.effect.frame;
  }
  return globalThis.style[property];
};
const root = { getAnimations: options => (options && options.subtree ? globalThis.running : []) };
const names = () => globalThis.running.map(animation => animation.name);
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
