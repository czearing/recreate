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

When several tabs match the same URL, pass the exact target ID from
`http://localhost:9222/json/list`:

```powershell
node src/extract.mjs --reuse --target <target-id> --out site-spec-output
```

Ambiguous matches fail immediately instead of choosing an arbitrary stale tab.
Every CDP command also has a 30-second deadline.

Capture internal routes, panels, and other state-changing controls:

```powershell
node src/extract.mjs --reuse --match example.com --crawl --max-routes 30 --out site-spec-output
```

Run destructive editor probes only against a disposable or explicitly approved
editor target:

```powershell
node src/extract.mjs --reuse --target <target-id> --crawl --editor-probes --out site-spec-output
```

Clipboard paste is a separate opt-in because it temporarily uses the system
clipboard:

```powershell
node src/extract.mjs --reuse --target <target-id> --crawl --editor-probes --clipboard-probe --out site-spec-output
```

The prior text clipboard value and clipboard permission setting are restored
after the probe. Editor probes are disabled by default for authenticated tabs.

Explicitly allow app transitions to another path subtree on the same host:

```powershell
node src/extract.mjs --reuse --match example.com --crawl --allow-cross-scope --out site-spec-output
```

Set explicit viewports:

```powershell
node src/extract.mjs --url https://example.com --out site-spec-output --viewports 1440x900,390x844
```

Capture screenshots only when debugging a structured-data gap:

```powershell
node src/extract.mjs --url https://example.com --out site-spec-output --screenshots
```

## Output

- `implementation.json`: concise agent-facing build plan and evidence index
- `spec.json`: machine-readable evidence index and global relationships
- `evidence/capture-*.json`: exact geometry and behavior for one viewport
- `evidence/state-*.json`: exact geometry, styles, focus, text, and
  content-addressed asset references for each interaction state and viewport
- `summary.json`: timings, counts, coverage, and validation
- `documents/`: exact pre-execution HTML responses
- `stylesheets/`: authored CSS source
- `pages/`: captured route/panel HTML and CSSOM
- `snapshot-assets/`: content-addressed images shared by captured states
- `component-map.json`: component hierarchy, identity, and DOM paths

The `full` profile also writes scripts, isolated component payloads, initial
documents, and screenshots. Default validation uses structured DOM, geometry,
styles, assets, and state transitions. Screenshots and pixel comparison are
optional diagnostics only; any mismatch must be resolved by capturing the
missing structured fact instead of asking an agent to interpret an image.

Implementation agents should read `implementation.json` first, then open only
the HTML, CSS, component, or exact geometry needed for the state currently
being built. The default profile excludes minified scripts, source excerpts,
inline asset encodings, repeated computed-style maps, and full scroll DOM
snapshots from the primary context path.

Large component and responsive collections are indexed instead of embedded in
`implementation.json`. Agents open `component-map.json`,
`evidence/responsive.json`, and one state/viewport evidence file only when the
active component requires them.

## Agent-context benchmark

| Fixture | Extraction | Agent entrypoint | Exact spec index |
| --- | ---: | ---: | ---: |
| Responsive docs, 2 viewports | 7.44 s | ~2.2k tokens | 14.7 KB or less |
| Commerce SPA, 2 viewports + crawl | 11.76 s | ~3.6k tokens | 14.7 KB |

The commerce capture retained its route and modal states, rebuilt successfully,
and contained no base64, data URLs, script excerpts, or minified script blobs
in the default text artifacts.

The crawler probes text submission, toggles, buttons, hash routes, delete
actions, clear actions, and double-click edit entry. Each resulting state stores
the action sequence that produced it, so build agents reproduce observed
behavior rather than infer it from control names.

Contenteditable editors add trusted reset, typing, selection, paragraph,
slash-menu, formatting, and clipboard probes. State evidence includes focus,
selection paths and offsets, selected text, and range rectangles.

Newly visible dialogs, menus, listboxes, trees, tooltips, and native popovers
are captured as modal or overlay states even when a portal places them outside
the trigger subtree. Form crawls prioritize the primary validation submit over
individual required fields and nested utility forms.

Native top-layer evidence includes reflected `open`, `popover`, and
`popovertarget` attributes, top-layer membership, computed `::backdrop` color,
filter, and opacity, focused content, plus trusted Escape dismissal and focus
return to the trigger.

Controls whose `aria-describedby` target has `role="tooltip"` receive a trusted
pointer-hover probe. The tooltip state records first-visible time,
fully-opaque time, exact geometry/styles, and pointer-leave dismissal duration
without turning the hover into a click or focus interaction.

Tooltip probing is opt-in so it does not consume route-state budgets:

```powershell
node src/extract.mjs --reuse --target <target-id> --crawl --tooltip-probes --out site-spec-output
```

Virtualized listboxes using `aria-activedescendant` can be probed explicitly:

```powershell
node src/extract.mjs --reuse --target <target-id> --crawl --virtual-list-probes --out site-spec-output
```

The resulting state records logical item count, mounted rows before and after
keyboard navigation, active-descendant identity, scroll-window replacement,
focus, and exact visible option geometry without inventing unmounted DOM.

Interaction states include sanitized Document, Fetch, and XHR timing from the
trigger until the network becomes quiet. Query keys are retained while query
values and encoded data/blob resources are excluded.

When a trigger changes the DOM while a request is still in flight, the crawler
captures a fast `transient` checkpoint before waiting for the final state. This
preserves loading, optimistic, and pending UI without letting the normal visual
settling delay erase it. Transient evidence is recorded at the primary viewport;
the final settled state retains the normal responsive evidence contract.

Open shadow roots are included in state evidence, interaction discovery,
post-click control fingerprints, modal/overlay detection, deep focus identity,
and trusted Escape dismissal. Closed roots remain explicitly unavailable.
Keyboard-driven games are probed from a clean state when page controls and copy
identify a bounded arrow-key or WASD interaction.

Implementation evidence excludes cross-frame internals, advertising/consent
subtrees, duplicate listeners, and known third-party ad listeners. Full profile
output retains the unfiltered forensic data.

CSS-linked images and fonts plus referenced GLB, glTF, HDR, KTX2, Basis, and
binary model resources are downloaded into `snapshot-assets/`. Stylesheet and
state references are rewritten to those content-addressed files.

WebGL evidence records context attributes, drawing-buffer dimensions, resource
timing, localized model assets, and two compositor-derived numeric samples.
The temporary compositor pixels are reduced to a small RGBA grid and hash; no
screenshot is written or supplied as implementation input.

Stable DOM is not sufficient acceptance by itself. Prominent 404 and
access-denied shells fail validation even when readiness and geometry succeed.

## Blind Next.js reproduction

The MIT-licensed TodoMVC React demo was captured without screenshots or source
code, then handed to a generation agent that could read only the structured
specification. The resulting clean Next.js 16 + TypeScript repository includes
Storybook stories and normal reusable components.

| Result | Measurement |
| --- | ---: |
| Capture time | 30.78 s |
| Captured states | 9 |
| Captured interactions | 8 |
| Viewports per state | 2 |
| Structured nodes validated | 948 / 948 |
| Geometry/style/text/focus failures | 0 |
| Screenshots used | 0 |

The reproduced states cover empty, add, toggle-all, clear-completed, edit entry,
delete, and All/Active/Completed hash routes at 1440×900 and 390×844. Tests,
lint, type-check, the production Next.js build, and the Storybook build pass.
Uncaptured edit commit/cancel behavior and persistence remain explicitly
unsupported instead of being invented.

## Multi-site component matrix

The extractor was iterated through isolated native translations before
increasing scope:

| Site class | Isolated proof | Evidence exercised |
| --- | --- | --- |
| Stateful todo app | Input and populated item | form, focus, hash routes, responsive state |
| Existing enterprise component library | Repeated card and controls | exact asset, responsive width, edit, menu, design-system overrides |
| Commerce SPA | Hero and tour dialog | image asset, modal, query route, desktop/mobile reflow |
| Documentation site | Navigation and article | sticky layout, typography, code blocks, anchors |
| Open-source keyboard game | Score header before/after key | trusted keyboard state, grid geometry, font assets, ad filtering |

Each proof has an isolated Storybook state and structured geometry/style
comparison. Failures were treated as extractor or translation defects rather
than masked with screenshot interpretation.

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
