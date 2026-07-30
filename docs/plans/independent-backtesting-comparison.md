# Independent Recreate Backtesting Comparison

## Status

Implemented under `backtest\`.

The executable is `recreate-backtest`. It is not a Recreate subcommand and does
not import Recreate capture, generation, verification, model, or browser code.

## Contract

`compare` has one 4,800 ms internal deadline and must finish below 5,000 ms.

Results:

- `PASS`: complete evidence and no differences
- `FAIL`: complete evidence with differences
- `INCONCLUSIVE`: deadline or unsupported runtime behavior blocked comparison
- `PREPARATION_REQUIRED`: invalid artifact, session, browser, or environment

Partial evidence cannot pass.

## Separation

`backtest\` has its own:

- `Cargo.toml`
- `Cargo.lock`
- workspace root
- executable
- CDP client
- browser launcher
- profiles
- session schema
- source artifact
- comparison engine
- report renderer
- fixture server
- tests and qualification evidence

The root Recreate workspace does not include `backtest\`. The package has no
path dependency and treats a selected Recreate binary as an opaque child
process.

Architecture tests recursively scan the backtest source tree, root workspace
membership, manifest dependencies, and independent lockfile. Qualification
also verifies that its owned browser processes terminate after the run.

## Commands

```text
recreate-backtest prepare source --url <url> --output <session.json>
recreate-backtest prepare candidate --url <url> --output <session.json>
recreate-backtest record --session <source-session.json> --output <source.json>
recreate-backtest compare --source <source.json> --candidate <session.json> --output <dir>
recreate-backtest benchmark --source <source.json> --candidate <session.json> --repeat 20
recreate-backtest qualify --fixtures <dir> --repeat 20
recreate-backtest pipeline --source-url <url> --candidate-url <url> --output <dir>
```

`prepare` opens a visible isolated browser by default. `--headless` is limited
to unattended local preparation.

## Authentication

Source and candidate use separate browser processes and profile directories.
Authentication remains inside those profiles.

The backtester does not:

- request credentials
- read cookies
- serialize tokens
- copy source authentication to the candidate origin
- implement application-specific login logic

Workflow:

1. Prefer attaching `prepare source --cdp-url <url> --target <id>` to the exact
   authenticated target returned by `recreate open`.
2. If no Recreate target exists, run `prepare source` and complete access in
   the visible source browser.
3. Run `record`; it reloads the requested route using the retained target and
   profile without copying or serializing authentication.
4. Run `prepare candidate`.
5. Complete candidate access independently.
6. Run `compare`.

Session and artifact files are integrity sealed. Corruption fails closed.

## Slow pages and deterministic time

Authentication, browser startup, source recording, Recreate generation, and
candidate warm-up occur before the timed comparison.

A pre-document script controls:

- timeouts
- intervals
- animation frames
- idle callbacks
- console errors
- rejected promises
- fetch requests

Source actions record virtual checkpoints. Candidate replay advances directly
to the same checkpoints. A 60-second timer therefore does not wait 60 seconds.

The comparison deadline wraps:

- target lookup
- CDP connection
- navigation
- readiness
- actions
- timer advancement
- capture
- artifact sealing

Expiration returns `INCONCLUSIVE`.

## Evidence

Each state records:

- viewport
- scenario
- source-owned semantic node identifiers
- tag, parent, and sibling order
- direct text
- visibility
- x, y, width, and height
- background and foreground color
- font weight
- border color and radius
- shadow, opacity, and transform
- role and accessible name
- animation duration, delay, easing, and direction
- CSS animation, CSS transition, and Web Animation identities
- normalized motion checkpoints at 0%, 25%, 50%, 75%, and 100%
- semantic layout topology: rows, columns, wrapping, order, gaps, and overlap
- unexpected layout-shift score and affected semantic elements
- active element
- console errors
- requests
- pending timers and frames
- baseline screenshot hash

Unknown pixel-only changes produce a viewport pixel finding when no stronger
semantic cause exists.

Layout comparison is based only on rendered rectangles and semantic roles.
Flex, grid, margins, classes, and DOM ancestry are not treated as differences
when they produce equivalent containment, alignment, spacing, and flow.
Repeated coordinate changes caused by one reflow are reduced to a single
topology finding.

Layout groups must also meet a rendered-density threshold. This prevents
unrelated controls scattered through one broad content region from being
misreported as a grid or toolbar. Animation identifiers and keyframe names are
treated as implementation details; equivalent rendered motion remains equal.

Motion comparison pauses and seeks browser animation timelines directly. It
does not wait for authored durations. CSS animations, transitions, and Web
Animations are compared by timing, easing, direction, iterations, affected
properties, and rendered values at bounded normalized checkpoints.

## Source-owned actions

The source declares independent test obligations with `data-backtest-action`.
Supported actions:

- click
- hover
- click sequence
- timer checkpoint
- animation checkpoint

Candidate markup cannot remove source actions because replay reads actions only
from the sealed source artifact.

## Difference selection

Each finding has a stable key:

```text
viewport + semantic target + property
```

Rules:

- one primary finding per root target
- earliest source scenario wins
- semantic causes outrank downstream x, y, height, transform, and pixel effects
- text outranks geometry
- visibility outranks layout movement
- animation timing outranks phase-shifted position
- repeated state symptoms are suppressed
- output order is stable

Text output is intentionally minimal:

```text
FAIL 1
V1440 base geometry-panel width 480->456 -24px
```

Full evidence remains in `comparison.json`.

## Isolated qualification corpus

```text
backtest/
  fixtures/
    comprehensive/
      source/
      control/
      mutations/
      expected.json
```

The control is behavior-identical to the source. Every mutation changes one
declared root behavior.

Covered mutations:

- missing node
- text
- width
- background color
- font weight
- responsive visibility
- click result
- hover paint
- focus restoration
- animation duration
- 60-second timer result
- accessibility role
- console error
- unexpected request

Each expected mutation defines:

- viewport
- scenario
- target
- property
- source value
- candidate value
- exact compact finding
- expected finding count
- duplicate count
- runtime ceiling

## Qualification gates

Control:

```text
status = PASS
findings = 0
duplicates = 0
duration < 5000ms
```

Single mutation:

```text
status = FAIL
primary findings = 1
unexpected findings = 0
duplicates = 0
finding = exact
duration < 5000ms
```

Corpus:

- mutation detection: 100%
- false passes: 0
- control false positives: 0
- duplicate keys: 0
- unstable normalized reports: 0
- p95: at most 4,000 ms
- p99: at most 4,500 ms
- maximum: below 5,000 ms

## Current measured evidence

Release qualification:

- comparisons: 300
- passed comparisons: 300
- detected mutations: 14/14
- duplicate findings: 0
- p95: 2,124 ms
- p99: 2,550 ms
- maximum: 2,650 ms

Warm standalone `compare` process:

- runs: 5
- average: 1,696 ms
- maximum: 1,855 ms

Raw evidence is written to the ignored paths
`backtest\target\qualification.json` and
`backtest\target\direct-process-timing.json`.

## Validation order

1. `cargo fmt --all -- --check`
2. `cargo test`
3. `cargo clippy --all-targets -- -D warnings`
4. `cargo build --release`
5. one-pass fixture qualification
6. repeat-20 fixture qualification
7. direct warm `compare` process timing
8. root workspace isolation check
9. final diff and artifact hash audit
