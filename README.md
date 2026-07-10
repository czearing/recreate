# site-spec

Extract a buildable website specification from browser ground truth through the Chrome DevTools Protocol.

The extractor records the exact initial document response, loaded DOM, authored CSS, scripts, assets, computed constraints, accessibility, listeners, responsive states, startup animation trajectories, scroll checkpoints, and component packages. It does not use image interpretation to infer implementation.

## Run

Start Edge with remote debugging enabled:

```powershell
msedge.exe --remote-debugging-port=9222 --user-data-dir="$env:TEMP\site-spec-edge"
```

Capture a public site:

```powershell
node src/extract.mjs --url https://example.com --out site-spec-output
```

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
- `stylesheets/` and `scripts/`: authored source blobs
- `pages/`: captured route/panel HTML, CSSOM, and screenshots
- `components/`: isolated component packages
- `component-map.json`: hierarchy and package index

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
