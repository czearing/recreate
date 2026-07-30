# Recreate Backtest

Independent browser backtesting for Recreate output.

## Build

```powershell
cargo build --release
```

## Qualify

```powershell
.\target\release\recreate-backtest.exe qualify `
  --fixtures .\fixtures `
  --output .\target\qualification.json `
  --repeat 20
```

## Compare prepared pages

Reuse an authenticated target already opened by Recreate:

```powershell
recreate open https://source.example --cdp-url http://127.0.0.1:9223

.\target\release\recreate-backtest.exe prepare source `
  --url https://source.example `
  --cdp-url http://127.0.0.1:9223 `
  --target <target-id-from-recreate-open> `
  --output .\target\source.session.json
```

This attaches to the exact live target and persistent Recreate browser profile.
It does not launch another browser, copy credentials, or require another
authentication step. `record` performs its instrumented reload in that same
target and profile.

To use a new isolated profile instead:

```powershell
.\target\release\recreate-backtest.exe prepare source `
  --url https://source.example `
  --output .\target\source.session.json

.\target\release\recreate-backtest.exe record `
  --session .\target\source.session.json `
  --output .\target\source.artifact.json

.\target\release\recreate-backtest.exe prepare candidate `
  --url http://127.0.0.1:5173 `
  --output .\target\candidate.session.json

.\target\release\recreate-backtest.exe compare `
  --source .\target\source.artifact.json `
  --candidate .\target\candidate.session.json `
  --output .\target\comparison
```

Focus on one semantic region or named element:

```powershell
.\target\release\recreate-backtest.exe compare `
  --source .\target\source.artifact.json `
  --candidate .\target\candidate.session.json `
  --focus "toolbar" `
  --output .\target\toolbar-comparison

.\target\release\recreate-backtest.exe compare `
  --source .\target\source.artifact.json `
  --candidate .\target\candidate.session.json `
  --focus "App launcher" `
  --output .\target\app-launcher-comparison
```

Focus is case-insensitive and matches captured semantic regions, roles, names,
and text. It fails with `PREPARATION_REQUIRED` unless the query exists on both
pages, preventing a typo or missing element from producing a false pass. The
full page is still captured under the same deadline; only the actionable report
is scoped.

Use `record --baseline-only` when an authenticated page has no stable,
source-authored interaction obligations and generic control replay would be
unsafe. The default remains full interaction recording.

Authentication remains in separate visible browser profiles. The comparison
deadline is 4,800 ms and incomplete evidence cannot pass.

## Rendered layout and motion

Live pages are compared by rendered intent rather than CSS implementation:

- semantic regions and controls
- rows, columns, wrapping, visual order, and overlap
- median row and column gaps
- typography and paint
- rendered `<img>` and inline SVG content, independent of URL or encoding
- attributed CSS background, pseudo-element, canvas, video, and effect pixels
- CSS animations, CSS transitions, and Web Animations timing
- rendered motion checkpoints at 0%, 25%, 50%, 75%, and 100%
- unexpected layout-shift score and affected elements

Equivalent flex, grid, margin, and positioning implementations compare equal
when they produce the same rendered relationships. Coherent child movement is
collapsed into one flow or gap root cause. Animation checkpoints are reached by
seeking browser animation timelines, not by sleeping.
Reports include every actionable finding and every grouped item name; output is
never truncated to a fixed number of differences.

The internal comparison deadline is 4,400 ms, leaving 400 ms to persist an
`INCONCLUSIVE` report before the process watchdog exits at 4,800 ms.

## Isolation gates

Automated architecture tests require:

- a standalone `backtest` workspace
- exclusion from the root Recreate workspace
- no path dependencies
- no `recreate` or `recreate-browser` packages in the backtest lockfile
- no Recreate namespace imports anywhere under `backtest\src`
- legacy artifacts to omit empty extended evidence and retain valid digests
- structured `INCONCLUSIVE` output to be written before command failure

Adversarial controls verify that equivalent flex/grid layouts, reordered DOM,
absolute/fixed positioning, renamed keyframes, and equivalent image encodings
do not create findings.
Scattered controls are excluded from inferred layout groups so unrelated
position changes remain individual findings.

Prepared standalone browsers intentionally persist so later commands can reuse
their CDP target. Qualification browsers are owned by the qualification process
and are terminated when it exits.
