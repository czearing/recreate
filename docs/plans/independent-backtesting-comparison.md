# Independent Recreate Backtesting Comparison Plan

## Status

Proposed implementation specification.

The implementation name is `recreate-backtest`. It is a separate tool for
testing Recreate output. It is not a new mode inside the existing `recreate`
binary.

## Objective

Build a browser-authoritative backtesting tool that:

1. Records a source page as immutable browser evidence.
2. Runs a selected Recreate binary as an opaque child process.
3. Serves the generated candidate in an isolated environment.
4. Compares source and candidate visual, content, interaction, animation, and
   asynchronous behavior.
5. Produces deterministic, terse, directly debuggable differences.
6. Always completes the timed comparison phase in less than five seconds.
7. Reports `PASS` only when every mandatory obligation was evaluated.

The tool must detect Recreate regressions without importing, reusing, or
trusting Recreate's capture, generation, verification, browser, model, or
comparison implementation.

## Hard guarantees

### Runtime guarantee

The `recreate-backtest compare` process must terminate within 5,000 ms.

The internal deadline is 4,800 ms, leaving 200 ms for cancellation, report
serialization, flushing files, and process exit. Every CDP request, browser
operation, comparison task, and artifact write receives the same absolute
deadline. Work still running at 4,800 ms is cancelled.

The five-second measurement begins only after source and candidate preparation
has completed. Authentication, initial source recording, Recreate generation,
dependency installation, browser startup, and uncontrolled network loading are
preparation operations and are not hidden inside the timed comparison.

Calling `compare` without valid prepared inputs fails immediately with
instructions to run `prepare`; it must never silently wait for login, browser
startup, a build, or a slow remote page.

### Accuracy guarantee

`PASS` means exact equivalence for the complete declared evidence envelope:

- browser build and operating environment
- viewport, device scale, locale, color scheme, and motion preference
- authenticated page state and route
- stable and animated pixels
- DOM, shadow DOM, text, attributes, pseudo-elements, and accessibility state
- geometry, scrolling, computed style, assets, and fonts
- source-derived interaction obligations
- animation timing, keyframes, progress states, and resulting geometry
- timers, microtasks, observers, network effects, console errors, and state
  transitions exercised by the evidence contract

If any mandatory evidence cannot be evaluated, the result is `INCONCLUSIVE`,
not `PASS`.

Universal equivalence for arbitrary JavaScript is not claimable. The tool
certifies the recorded source-derived behavior envelope and exposes every
unsupported or uncovered obligation.

## Protected separation from Recreate

The implementation will live in a standalone top-level `backtest\` package:

```text
backtest/
  Cargo.toml
  Cargo.lock
  src/
  fixtures/
  tests/
```

It will produce a separate executable:

```text
recreate-backtest
```

The package must have its own:

- Cargo manifest and lockfile
- CLI and artifact schema
- CDP client and browser launcher
- source recorder and candidate probe
- browser profiles and ports
- comparison engine and report renderer
- fixtures, tests, benchmarks, and release artifact

It must not:

- add a `recreate backtest` subcommand
- import the root Recreate crate
- import `recreate-browser`
- import files or generated modules from `src\`
- call Recreate verification or fidelity functions
- accept Recreate's `spec.json` or `acceptance.json` as truth
- let candidate output choose, remove, or weaken source obligations
- change existing Recreate capture, generation, installation, or update behavior

A dependency-boundary test will inspect Cargo metadata and fail if the
backtester depends on any Recreate workspace package or source path.

## Backtesting Recreate as a black box

The backtester receives the exact Recreate executable to test:

```text
recreate-backtest pipeline \
  --recreate-bin C:\path\to\recreate.exe \
  --source-session source.session.json \
  --candidate-command "npm run dev" \
  --out backtest-results\
```

The pipeline will:

1. Validate the prepared source session and evidence artifact.
2. Launch the selected Recreate binary as a child process.
3. Give it only documented command-line inputs.
4. Capture its exit code, stdout, stderr, duration, and generated file hashes.
5. Build and serve its generated candidate in a disposable directory.
6. Prepare the candidate browser target.
7. Start the isolated sub-five-second comparison.
8. Attribute detected regressions to the tested Recreate binary and artifact
   hashes.

The generation and build durations are reported separately. They are not
included in the five-second comparison claim.

This enables backtesting:

- two Recreate releases against the same source artifact
- a branch binary against the last passing release
- repeated candidates against one immutable authenticated source recording
- generated-output mutations that verify defect detection sensitivity

## Authentication and protected pages

### Authentication rule

The backtester never asks for, reads, exports, logs, or stores credentials,
cookies, authorization headers, local storage values, or tokens.

Authentication happens in dedicated visible browser windows owned by
`recreate-backtest`.

```text
recreate-backtest prepare source https://source.example \
  --session source.session.json

recreate-backtest prepare candidate http://127.0.0.1:5173 \
  --session candidate.session.json
```

Source and candidate use separate browser processes and profiles:

```text
%USERPROFILE%\.recreate-backtest\profiles\source\
%USERPROFILE%\.recreate-backtest\profiles\candidate\
```

The user completes any login, consent, MFA, or access step inside the exact
visible target. The tool then confirms that the requested interface is visibly
rendered before recording the session.

### Session manifest

The session manifest contains non-secret coordination data only:

- schema version
- profile identifier
- CDP endpoint
- target identifier
- requested and rendered origins
- route
- browser build
- viewport and environment contract
- visible-interface fingerprint
- preparation timestamp
- evidence artifact hash

The browser profile remains the only owner of authentication state.

Before each comparison, the tool verifies the rendered interface fingerprint.
An expired session, access redirect, consent page, or mismatched account state
invalidates preparation. It does not consume the comparison budget and cannot
be mistaken for a visual regression.

### Candidate authentication

The candidate is authenticated independently because source-origin cookies
must not be copied to another origin.

Candidate preparation may use:

- a visible manual login in its dedicated profile
- a developer-provided local test account entered in-browser
- a local application fixture or documented test mode
- recorded network fixtures when the comparison contract permits replay

The backtester does not implement application-specific login logic.

## Slow loading and long-running behavior

### Two-phase execution

Slow pages are handled by separating preparation from comparison.

#### Preparation phase

Preparation may take as long as required. It:

- starts the dedicated browser
- completes authentication
- waits for the requested interface
- records response bodies allowed by the evidence policy
- records fonts, images, stylesheets, and asset hashes
- records timers, animation registrations, observers, and event listeners
- records source-derived interaction obligations
- records startup, loading, ready, and post-ready checkpoints
- records nondeterministic or unsupported behavior
- writes an integrity-hashed source artifact

#### Timed comparison phase

Comparison starts from warm prepared targets and does not wait for real time.
It advances deterministic browser state and compares checkpoints in memory.

### Readiness state machine

Readiness is not `document.readyState` and is not a fixed sleep.

The source recorder defines a browser-native checkpoint state machine:

1. `navigation-committed`
2. `document-created`
3. `interface-identified`
4. `critical-assets-resolved`
5. `interaction-surface-stable`
6. `async-checkpoint-reached`

`interaction-surface-stable` requires:

- the semantic target exists
- its geometry is non-zero
- hit testing reaches the target or an owned descendant
- it is not covered by a blocking surface
- its enabled, focusable, and accessibility state matches the source obligation
- its identity and geometry remain equal across consecutive virtual
  checkpoints

The comparison never clicks a control merely because a wall-clock delay ended.

### Virtual time

Timers and scheduling are driven through Chromium virtual time:

- `setTimeout` and `setInterval`
- `requestAnimationFrame`
- microtask checkpoints and resolved promises
- `requestIdleCallback`
- mutation delivery
- resize and intersection observer checkpoints
- deferred UI state driven by recorded network responses

The tool advances directly to source-observed checkpoints, such as virtual
`0 ms`, `16 ms`, `100 ms`, `500 ms`, `2 s`, or `60 s`, without waiting those
durations in real time.

CPU-bound JavaScript still consumes real CPU. If it cannot complete inside the
deadline, the result is `INCONCLUSIVE` with a CPU-bound execution diagnostic.

### Animations

Long animations never block interaction discovery.

The recorder captures:

- target identity
- keyframes and animated properties
- duration, delay, iterations, direction, fill, easing, and timeline type
- layout and paint effects
- start and end event effects

The comparer pauses animations and seeks them to normalized progress points:

```text
0%, 25%, 50%, 75%, 100%
```

Additional points are added at source-observed discontinuities, keyframe
boundaries, interaction-enablement changes, and animation events.

Infinite animations are compared over one canonical cycle plus their iteration
contract. They never run indefinitely.

### Async network behavior

Preparation records an allowlisted response fixture set and request contract.
Requests are normalized by method, route, body hash, relevant headers, and
semantic sequence rather than hostname alone.

During comparison:

- deterministic source responses may be replayed to both sides
- request order and payload differences are still reported
- unexpected candidate requests are failures
- missing candidate requests are failures
- live responses that were not recorded are coverage obligations
- WebSocket, streaming, service-worker, and push behavior require explicit
  recorded protocols or produce `INCONCLUSIVE`

A genuinely slow live network is never allowed to consume the five-second
comparison budget.

### Distinguishing slow from broken

The result classifications are:

- `FAIL`: the candidate reached the equivalent virtual checkpoint but produced
  different pixels, content, structure, state, requests, errors, or behavior
- `INCONCLUSIVE`: the checkpoint could not be reproduced because evidence was
  missing, unsupported, nondeterministic, CPU-bound, opaque, or expired
- `PREPARATION_REQUIRED`: authentication, browser startup, source recording,
  candidate build, or candidate warm-up has not completed

The tool never converts a timeout into a visual mismatch and never converts
incomplete loading into a pass.

## Source-derived interaction obligations

All test obligations are discovered from the source before candidate
comparison. Candidate markup cannot suppress tests.

The recorder observes:

- actionable accessibility roles and native controls
- registered event listeners
- pointer, hover, focus, keyboard, input, selection, scroll, and drag semantics
- dialogs, menus, listboxes, tooltips, overlays, carousels, and dismissals
- enabled and disabled transitions
- focus movement and restoration
- repeated activation and inverse actions
- responsive state changes
- timers, observers, network completions, and animation events that alter UI

Each obligation includes:

- stable source identity
- precondition checkpoint
- trusted input sequence
- virtual-time checkpoints
- expected state transition
- expected DOM, pixel, accessibility, focus, network, and console effects
- restoration path
- evidence provenance

Independent candidate matching uses semantics, geometry, accessibility, and
source relationships. It does not rely solely on generated DOM paths.

## Capture and comparison evidence

At every mandatory checkpoint the backtester records:

- screenshot and stable-region pixel hashes
- flattened DOM and shadow DOM
- exact text-node order and content
- attributes and pseudo-elements
- complete computed style values needed by visible nodes
- element rectangles, client rectangles, scrolling, and document geometry
- accessibility tree and focus state
- animation descriptors and normalized progress state
- asset, font, and decoded image hashes
- request and response contracts
- console errors, exceptions, and failed resources
- state-changing mutations and observer effects

Stable pixels require exact equality in the pinned environment. Dynamic regions
are not broadly masked. They are compared at equivalent deterministic
checkpoints.

## Difference report

The tool emits:

```text
backtest-results\
  comparison.txt
  comparison.json
  evidence\
  images\
```

`comparison.txt` contains only:

```text
FAIL 2 842ms
V1440 click:sign-in dialog width 480->456 -24px
V390 base nav text "Pricing"->"Plans"
```

Line grammar:

```text
V<width> <action> <target> <property> <source>-><candidate> <delta?>
```

Text rules:

- one line per root difference
- no paragraphs
- no duplicate labels
- no repeated symptoms
- no likely-cause prose
- no confidence prose
- no evidence paths
- omit unchanged context
- omit zero deltas
- stable order
- maximum 120 characters per line

The canonical difference key is:

```text
scenario + checkpoint + viewport + source-obligation +
semantic-target + earliest-property + cause-class
```

Only the earliest causal difference for a key is emitted. Wrapping, movement,
clipping, and descendant paint changes caused by that difference become
secondary evidence in `comparison.json`, not extra text lines.

`comparison.json` retains:

- stable difference identifier and canonical key
- category and severity
- viewport and checkpoint
- reproduction action
- semantic source and candidate locators
- source and candidate values
- measured delta
- dependent effects
- evidence paths
- confidence

Output bytes must be stable across repeated runs.

## Five-second execution budget

| Stage | Maximum |
| --- | ---: |
| Validate prepared artifacts and connect to warm targets | 150 ms |
| Restore deterministic baseline checkpoints | 650 ms |
| Capture source and candidate baseline evidence concurrently | 900 ms |
| Execute interaction and async scenario shards concurrently | 1,900 ms |
| Hash, compare, and group root causes | 750 ms |
| Serialize artifacts and terminate work | 450 ms |
| Reserved cancellation margin | 200 ms |
| **Total** | **5,000 ms** |

The implementation target is:

- warm p50 at or below 3.0 seconds
- warm p95 at or below 4.0 seconds
- warm p99 at or below 4.5 seconds
- absolute process termination below 5.0 seconds

If the mandatory scenario set cannot fit, preparation must shard and precompute
source evidence. The timed command may parallelize candidate scenario targets
but may not silently drop obligations.

## Command surface

```text
recreate-backtest prepare source <url> --session <path>
recreate-backtest prepare candidate <url> --session <path>
recreate-backtest record --session <source-session> --out <artifact>
recreate-backtest compare <artifact> --candidate-session <session> --out <dir>
recreate-backtest pipeline --recreate-bin <path> --source-session <session> --out <dir>
recreate-backtest benchmark <artifact> --candidate-session <session>
recreate-backtest qualify --fixtures <dir>
```

`prepare` and `record` are intentionally outside the five-second contract.
`compare` is always inside it. `pipeline` reports generation, build,
preparation, and comparison timings separately.

## Implementation phases

### Phase 1: Independent skeleton

- Add standalone `backtest\` package and binary.
- Add dependency-boundary and root-CLI-unchanged tests.
- Add independent CDP transport, browser launcher, profiles, and target
  lifecycle.
- Add the absolute deadline and cancellation infrastructure.

### Phase 2: Authentication and preparation

- Add visible source and candidate preparation commands.
- Add non-secret session manifests and rendered-interface fingerprints.
- Add stale-session, redirect, access-page, and profile-isolation validation.
- Add source artifact integrity hashes.

### Phase 3: Browser evidence

- Add DOM, pixel, accessibility, style, geometry, asset, network, console, and
  animation capture.
- Add source-only interaction and scheduler instrumentation.
- Add evidence provenance and completeness obligations.

### Phase 4: Deterministic execution

- Add virtual-time checkpoint control.
- Add animation pause and seek.
- Add network fixture replay.
- Add parallel scenario targets and prefix sharing.
- Add explicit unsupported-behavior classifications.

### Phase 5: Comparison and reporting

- Add exact checkpoint comparison.
- Add semantic candidate matching and causal difference grouping.
- Add deterministic JSON and terse line reports.
- Add canonical keys and duplicate suppression.
- Add `PASS`, `FAIL`, `INCONCLUSIVE`, and `PREPARATION_REQUIRED`.

### Phase 6: Recreate pipeline backtesting

- Invoke selected Recreate binaries only as child processes.
- Build and serve generated candidates in disposable directories.
- Record binary and generated artifact hashes.
- Compare releases, branches, and repeated candidates against immutable source
  artifacts.

### Phase 7: Qualification and performance

- Add paired local source, control, and single-mutation candidates.
- Add static, responsive, authenticated, animated, asynchronous, interaction,
  streaming, opaque-frame, and failure fixtures.
- Add deliberate generated-output mutations for every evidence category.
- Require exactly one expected primary finding per single mutation.
- Require zero findings for identical controls.
- Require zero duplicate canonical keys.
- Require byte-identical reports across repeated runs.
- Require unsupported cases to be `INCONCLUSIVE`.
- Enforce p95, p99, and absolute five-second gates in CI.

## Isolated paired-site corpus

All qualification sites are local, deterministic, and isolated from the
internet.

```text
backtest/
  fixtures/
    <case>/
      source/
      control/
      mutations/
        <mutation-id>/
      expected.json
```

Each case contains:

- `source`: canonical rendered site
- `control`: byte-separate but behavior-identical site
- `mutations`: candidate sites with one declared change each
- `expected.json`: exact expected result for every candidate

`source` and `control` must render identically but be served from separate
origins and processes.

Each single-mutation candidate changes exactly one root property or behavior.
Generated variants are preferred over copied sites so every changed byte is
declared.

Composite candidates are allowed only after every included single mutation
passes its isolated gate.

### Mutation manifest

```json
{
  "id": "dialog-width",
  "category": "geometry",
  "scenario": "click:sign-in",
  "viewport": 1440,
  "target": "dialog",
  "property": "width",
  "source": "480px",
  "candidate": "456px",
  "finding": "V1440 click:sign-in dialog width 480->456 -24px"
}
```

No test derives expected output from the candidate. The manifest is authored
before execution and validated against the source mutation.

### Required single mutations

| Category | Isolated changes |
| --- | --- |
| Structure | missing, extra, reordered, wrong parent, shadow boundary |
| Content | text, whitespace, attribute, pseudo-content, icon |
| Geometry | x, y, width, height, gap, wrapping, overflow, scroll |
| Paint | color, background, border, radius, shadow, opacity, image |
| Typography | font, weight, size, line-height, letter spacing |
| Responsive | breakpoint, visibility, order, wrapping, container width |
| Interaction | click, hover, focus, keyboard, input, close, restore |
| Animation | target, property, keyframe, duration, easing, direction |
| Async | timer, promise, observer, request, response, sequence |
| Accessibility | role, name, state, focus order, disabled state |
| Runtime | console error, exception, failed request, unexpected request |

### Per-candidate assertions

Identical control:

```text
status = PASS
findings = 0
duplicates = 0
duration < 5000ms
```

Supported single mutation:

```text
status = FAIL
primary findings = 1
expected primary findings = 1
unexpected primary findings = 0
duplicates = 0
target/property/source/candidate = exact
duration < 5000ms
```

Unsupported fixture:

```text
status = INCONCLUSIVE
coverage reason = exact
status != PASS
duration < 5000ms
```

Each candidate runs 20 times. `comparison.txt` and normalized
`comparison.json` must be byte-identical across all runs.

### Corpus gates

- seeded-mutation detection: 100%
- false passes: 0
- control false positives: 0
- duplicate canonical keys: 0
- wrong primary findings: 0
- unstable reports: 0
- warm p95: at most 4.0 seconds
- warm p99: at most 4.5 seconds
- maximum process time: below 5.0 seconds

## Required fixture behaviors

The qualification corpus must include:

- manual authenticated source and candidate sessions
- expired authentication and access redirects
- a 30-second CSS animation
- a 60-second JavaScript timer
- delayed promises and chained microtasks
- delayed fetch and reordered responses
- Web Animations API and CSS keyframes
- requestAnimationFrame-driven state
- mutation, resize, and intersection observers
- a control covered during startup and enabled later
- a modal with focus trapping and restoration
- hover, keyboard, input, scroll, carousel, and dismissal behavior
- responsive breakpoints and text wrapping
- fonts and images that finish after document load
- CPU-bound JavaScript
- opaque cross-origin frames
- WebSocket and service-worker behavior

## Acceptance criteria

Implementation is complete only when:

1. Existing Recreate CLI commands and behavior remain unchanged.
2. `recreate-backtest` has no Recreate code dependency.
3. Authentication data never leaves dedicated browser profiles.
4. Slow timers and long animations are evaluated without wall-clock waiting.
5. Interaction execution is gated by browser-native stability and hit testing.
6. Every mandatory source obligation is passed, failed, or explicitly
   inconclusive.
7. Partial evidence can never produce `PASS`.
8. Each supported single mutation produces one terse, exact, directly
   debuggable primary finding.
9. Supported seeded mutations are detected with no false pass.
10. Identical controls produce no findings and all reports contain no duplicate
    canonical keys.
11. Warm p95 is at most 4.0 seconds, warm p99 is at most 4.5 seconds, and every
    timed comparison process terminates below 5.0 seconds.
