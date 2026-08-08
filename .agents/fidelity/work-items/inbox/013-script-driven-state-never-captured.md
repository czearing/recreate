# WI-013 — Script-driven state is never captured: no trigger discovery exists on the capture path

- **Behavior name:** `disclosure-two-state` (WAI-ARIA disclosure: one button, one panel, delta is one attribute)
- **Mechanism family:** script-driven state
- **Scene:** `C:\Code\recreate-testing-scenes\scenes\disclosure-two-state`
- **Generated output:** `C:\Code\recreate-testing-scenes\project\dts-1` (also `dts-2`, `dts-3`)
- **Owning module:** `src/capture.rs` — the absent trigger-discovery stage (`Specification { interactions, transitions }` construction, line ~105)

## Reproduce

```
C:\Users\calebzearing\recreate\target\release\recreate.exe capture C:\Code\recreate-testing-scenes\scenes\disclosure-two-state --out C:\Code\recreate-testing-scenes\project\dts-1
C:\Users\calebzearing\recreate\target\release\recreate.exe capture C:\Code\recreate-testing-scenes\scenes\disclosure-two-state --out C:\Code\recreate-testing-scenes\project\dts-2
C:\Users\calebzearing\recreate\target\release\recreate.exe capture C:\Code\recreate-testing-scenes\scenes\disclosure-two-state --out C:\Code\recreate-testing-scenes\project\dts-3
```

All three runs are byte-identical (same `Baseline0.jsx` and CSS hashes). This is not ambient nondeterminism.

## Authored ground truth

The scene has **exactly two states by construction**, because both were authored:

- collapsed — `#toggle[aria-expanded="false"]`, `#panel[hidden]`
- expanded — `#toggle[aria-expanded="true"]`, `#panel` visible, and a visual delta authored **entirely** in CSS as `#toggle[aria-expanded="true"] { background:#1f6feb; border-color:#0b3d91; color:#fff; }`

No class toggle, no text change, no timer, no network, no clock- or random-derived value. The delta is one attribute on one element.

## Wrong output

`spec.json`:

```
"states": 5          <- the five viewports, not widget states
"interactions": []   <- count 0
"transitions": []    <- count 0
"capture_blockers": []
```

Every one of the five states carries `"aria-expanded":"false"`. The string `"aria-expanded":"true"` appears **nowhere** in any of the three specs.

`react/src/views/Baseline0.jsx` — a static snapshot with no handler:

```jsx
<button className={"r221b5bb45d"} aria-controls={"panel"} aria-expanded={"false"} id={"toggle"} type={"button"}>
<div className={"rf4c6f05d46"} hidden={true} id={"panel"}>
```

Generated CSS — the authored state rule is gone:

```
Baseline0.css: 0 matches for `aria-expanded`  (96 lines)
styles.css:    0 matches for `aria-expanded`  (118 lines)
```

...even though the rule **is** present in the captured evidence, in `states[].css_rules`:

```
#toggle[aria-expanded="true"] { ... }
```

and `states[].state_styles` is empty (count 0).

## Expected output

1. `spec.json` records the expanded state in addition to the collapsed one.
2. `spec.json` contains an `interactions[]` / `transitions[]` entry whose trigger resolves to `#toggle`.
3. Generated JSX binds a click handler that flips `aria-expanded`, and generated CSS retains a rule keyed on the `[aria-expanded="true"]` attribute selector.

All three fail today.

## Root cause — two boundaries, one root

The root is that **the capture path has no representation of script-driven state at all.** It shows up at two independent boundaries:

**A. Trigger discovery does not exist.** `src/capture.rs` builds the `Specification` with `interactions: Vec::new(), transitions: Vec::new()` and nothing in `capture()` ever populates them. `interaction_surface::normalize(&mut specification)` is called on the next line and iterates a vec that is always empty. `interactions_input::click_matching` / `prepare_interaction_state` / `read_interaction_state` exist but are reachable **only** from `compare_capture.rs` and `release_gate_tests::verify` — the replay/verification path, never the capture path. Consequence: no control on any site is ever exercised, so no second state can exist for any page, not just this one.

The downstream machinery is already built and is simply never fed: `generate/templates/app_component.jsx` already contains

```js
document.querySelectorAll('[data-recreate-trigger][aria-expanded]').forEach(element =>
  element.setAttribute('aria-expanded', String(Number(element.dataset.recreateTrigger) === state)))
```

so `interactions.rs`, `interaction_bindings.rs`, `jsx_state_*` and `css_interactions.rs` are all dead on a real capture.

**B. A state expressed as an attribute is not recognised as a state.** `src/rule_activation_script.rs:12`:

```js
const dynamicStatePattern = /:(hover|focus-visible|focus-within|focus|active)\b/g;
```

`state_style_script.rs::captureStateStyles` does `if (!states.length && !reduced) continue;`, so a rule keyed on an attribute selector is never recorded as a state style. Because no element currently matches `#toggle[aria-expanded="true"]`, `authored_css.rs` cannot attach it to a node either, so the authored rule is present in `css_rules` and then dropped from the output entirely. Script-driven state is expressed as attributes — `[aria-expanded]`, `[aria-pressed]`, `[data-state]`, `[open]`, `[hidden]`, `[disabled]`, `[checked]` — and none of them can ever reach a five-item pseudo-class list.

This is the shape the module's own doc comment already argues against: *"Whether an authored rule actually applies is a browser decision, not a parse decision."* One line below, selector state is decided by a hardcoded parse list.

## Fixed condition (binary)

Capturing `disclosure-two-state` produces a `spec.json` whose `interactions[]` contains an entry whose trigger resolves to `#toggle`, **and** the generated project contains both a bound click handler that flips `aria-expanded` on that element and a CSS rule keyed on `[aria-expanded="true"]`.

## Do not satisfy it this way

- **Do not special-case `aria-expanded`** (or `aria-pressed`, `open`, `hidden`, `data-state`, or any other attribute) **by name.** The next widget will use a different attribute. Whether a selector component describes a state must be derived — a component is a state component when the element matches the selector without it and does not match the selector with it. Let the browser decide, do not extend the parse list.
- **Do not lower a state-similarity threshold globally.** The states were never merged — `interactions: 0` proves nothing was ever exercised. Touching a threshold would not change this result and would cost state explosion everywhere else.
- **Do not hardcode a second state for this fixture**, and do not add any code path that reads the scene directory.
- **Do not emit a second state that is unreachable from any bound handler.** A state in `spec.json` that no generated handler can enter is worse than an omission, because it makes the evidence look correct while output stays static.
- **Do not fix this only in the generate stage.** The evidence for the interaction does not exist; anything added downstream would be invented.
- **Do not make `staticSelector` strip attribute selectors** to solve (B). That probe decides at-rule activation; stripping attributes there makes probes over-match and would record genuinely inert rules, regressing the `inactive-supports-block` expectation.

## Hypotheses killed

| Hypothesis | Verdict | Evidence |
|---|---|---|
| **States were merged by DOM-similarity abstraction below a threshold** (the predicted failure mode, registered before the first capture) | **Refuted** | `interactions: 0` and `transitions: 0`. `Interaction` is stored separately from the states it produced, so a merge would still leave the interaction record. Nothing was exercised, so nothing could be merged. |
| The handler never fired because the scene's script was broken (unkilled-mutant / attribution control) | Refuted | `capture_blockers: []` and the script loads; the tool never dispatches any click on the capture path at all, so no scene script could have been exercised. |
| The result is capture nondeterminism | Refuted | Three captures produced byte-identical JSX and CSS hashes. |
| `states: 5` means five widget states were found | Refuted | The five are the five viewports; every one has `aria-expanded=false` and `panel.hidden=true`. |
| `position:sticky` loss | Pre-refuted | Zero occurrences in the targets' `spec.json`. |
| 25 animations producing 0 `@keyframes` | Pre-refuted | Those are `window.__recreateLifecycleAnimations` geometry samples lacking `composite`/`easing`/`fill`; `sampled_layout_observation` discards them rightly. |
| Orphaned keyframes via `classes.entry(target).and_modify(...)` | Pre-refuted | Not real; the class is applied. |
| Running-animation sampled-frame drift | Already filed | WI-012. |

## Why this needed an authored scene

A negative control and a metamorphic twin both compare tool output against tool output, and an omission leaves no trace to compare — it survives any oracle that only inspects what the system reports. The authored fixture supplies independently-derived ground truth (the state count is known because both states were written by hand), and the assertions are stated as counts and presence checks rather than diffs. The attribution check (2) is what separated "exercised then merged" from "never exercised", and it is the check that overturned the predicted cause.
