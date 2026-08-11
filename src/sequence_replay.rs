//! Driving the shipped `runtime/sequence.mjs` against a fake element and a virtual clock.
//!
//! The runtime module is imported and executed rather than searched for its own words, and
//! time is supplied by the caller, so a scenario spanning simulated minutes runs in
//! milliseconds and never opens a browser.
//!
//! The element answers on whichever channel the sequence writes, because the questions asked
//! of a replay — where does it resume, when does it stop — are the same question whatever
//! kind of value is being written, and a harness that only speaks text can only ever prove
//! the text half.

use serde_json::Value;

/// The shipped runtime module, as a URL Node will import.
fn module() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("runtime")
        .join("sequence.mjs")
        .display()
        .to_string()
        .replace('\\', "/")
}

/// Replays `sequence` against an element captured holding `captured` on the channel the
/// sequence writes, running up to `ticks` scheduled steps.
///
/// Reports every value the element took, starting with the captured one, and whether the
/// replay left its queue empty: a terminating sequence stops with steps to spare, and a
/// looping one exhausts the budget.
pub fn replay(sequence: &str, attribute: &str, captured: &str, ticks: u32) -> Value {
    let module = module();
    crate::node_eval::evaluate(
        &format!(
            r#"import {{ startSequence }} from 'file:///{module}';
const kind = {attribute:?};
const seen = [];
const attributes = new Map();
const element = {{
  childNodes: [],
  setAttribute(name, value) {{
    attributes.set(name, value);
    if (name === kind) seen.push(value);
  }},
  getAttribute(name) {{ return attributes.has(name) ? attributes.get(name) : null; }}
}};
Object.defineProperty(element, 'textContent', {{
  get: () => (kind === 'textContent' ? seen.at(-1) : ''),
  set: value => {{ seen.push(value); }},
  configurable: true
}});
const captured = {captured};
if (captured !== null) {{
  seen.push(captured);
  if (kind !== 'textContent') attributes.set(kind, captured);
}}
let queued = null;
const clock = {{
  setTimeout: handler => {{ queued = handler; return 1; }},
  clearTimeout: () => {{ queued = null; }}
}};
startSequence(element, {sequence}, clock);
let ran = 0;
while (queued && ran < {ticks}) {{ const next = queued; queued = null; next(); ran++; }}
const stopped = queued === null;
"#
        ),
        "({ seen, stopped })",
    )
}

/// A cycle of three values, written to `attribute`, observed to come back round.
pub fn cycle(attribute: &str) -> String {
    format!(
        r#"{{
  attribute: '{attribute}', repeats: true,
  steps: [
    {{ value: 'Alpha', delay_ms: 300 }},
    {{ value: 'Bravo', delay_ms: 300 }},
    {{ value: 'Charlie', delay_ms: 300 }}
  ]
}}"#
    )
}
