# site-spec

Extract a buildable website specification from browser ground truth through the Chrome DevTools Protocol.

The extractor records the exact initial document response, loaded DOM, authored CSS, assets, computed constraints, listeners, responsive states, startup animation trajectories, scroll checkpoints, and component boundaries. It does not use image interpretation to infer implementation.

## Run

Start Edge with remote debugging enabled:

```powershell
msedge.exe --remote-debugging-port=9222 --user-data-dir="$env:TEMP\site-spec-edge"
```

Capture a public site:

```powershell
node src/extract.mjs --url https://example.com --out site-spec-output
```

The default `implementation` profile keeps the evidence needed to rebuild and
validate the site while omitting duplicated forensic payloads. Use
`--profile full` when raw DOMSnapshot documents, complete script blobs, and
standalone component payloads are required for debugging.

Capture an authenticated tab already open in that browser:

```powershell
node src/extract.mjs --reuse --match example.com --out site-spec-output
```

Capture internal routes, panels, and other state-changing controls:

```powershell
node src/extract.mjs --reuse --match example.com --crawl --max-routes 30 --out site-spec-output
```

Set explicit viewports:

```powershell
node src/extract.mjs --url https://example.com --out site-spec-output --viewports 1440x900,390x844
```

## Output

- `spec.json`: complete machine-readable specification
- `summary.json`: timings, counts, coverage, and validation
- `documents/`: exact pre-execution HTML responses
- `stylesheets/`: authored CSS source
- `pages/`: captured route/panel HTML, CSSOM, and screenshots
- `snapshot-assets/`: content-addressed images shared by captured states
- `component-map.json`: component hierarchy, identity, and DOM paths

The `full` profile also writes `scripts/` and isolated `components/` payloads.
Screenshots are ground-truth validation evidence. Pixel comparison belongs in
the final implementation-validation pass and is not run during extraction.

## Build an interactive static mock

```powershell
node build-static.mjs --spec site-spec-output --out site-spec-build
node site-spec-build/server.mjs 4317
```

The generated build maps captured controls to local route and panel states,
preserves browser back navigation, loads captured per-state CSS, and excludes
captured application scripts.

Captured output can contain source code and user-visible data. Keep it out of version control unless you have permission to publish it.

## Measured fixture

| Metric | Result |
| --- | ---: |
| Two-viewport extraction | 27.158 s |
| Opening animation duration | 3.4 s |
| Opening animation definitions | 41 |
| Startup tracks | 374 |
| Smooth-scroll range | 32,242 px |

The reconstruction proof booted from the captured initial HTML, completed the opening sequence naturally, initialized Lenis, and changed GSAP-controlled transforms during deep scrolling.
