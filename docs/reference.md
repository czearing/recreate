# Recreate

Capture a live interface as structured browser ground truth for native reconstruction and validation.

The extractor records the exact initial document response, loaded DOM, authored CSS, assets, computed constraints, listeners, responsive states, startup animation trajectories, scroll checkpoints, and component boundaries. It does not use image interpretation to infer implementation.

## Install

Install the personal Recreate skill for detected GitHub Copilot CLI and Claude
Code clients:

```text
npx --yes --registry=https://registry.npmjs.org/ recreate-cli@latest install
```

Install for an explicit client when auto-detection is not available:

```text
npx --yes --registry=https://registry.npmjs.org/ recreate-cli@latest install --copilot
npx --yes --registry=https://registry.npmjs.org/ recreate-cli@latest install --claude
npx --yes --registry=https://registry.npmjs.org/ recreate-cli@latest install --all
```

The installed skill resolves `recreate-cli@latest` with npm online checks on
every use, so the skill does not need to be reinstalled after a release.

### What gets installed

| Client | Personal skill path |
| --- | --- |
| GitHub Copilot CLI | `~/.copilot/skills/recreate/SKILL.md` |
| Claude Code | `~/.claude/skills/recreate/SKILL.md` |

The installer is idempotent. Running it again refreshes the Recreate skill
without changing other skills or client settings.

### Use the skill

The installed skill is named `recreate`. Invoke it directly:

```text
/recreate Capture https://example.com and rebuild it in this project.
```

The skill handles the workflow:

1. Loads the current Recreate instructions from npm.
2. Reuses or starts Chrome, Edge, or Chromium with remote debugging.
3. Briefly inspects the rendered page before capture.
4. Resolves access-page ambiguity with the user when necessary.
5. Captures the exact inspected tab and session.
6. Guides native implementation from the captured evidence.
7. Checks the result against the captured acceptance matrix.

### Access pages

The skill makes the access decision from the rendered page as a whole. It does
not use hardcoded words, selectors, URL patterns, or account-control detection.
Raw HTML and HTTP fetches are not used as a substitute for the browser view.

If the page is clearly an access step in front of the requested interface, the
skill asks the user to complete access in the browser tab that is already open.
It then inspects and captures that same tab.

If the page itself may be the interface the user wants recreated, the skill
asks which page is intended before capturing anything. Credentials and session
data remain in the browser.

### Automatic updates

The installed `SKILL.md` is a small launcher. Every invocation runs:

```text
npx --yes --prefer-online --registry=https://registry.npmjs.org/ recreate-cli@latest skill
```

`@latest` selects the current stable release. `--prefer-online` forces npm to
check the registry instead of trusting a cached package version. New releases
are used on the next invocation. No reinstall is required.

### Verify the installation

For GitHub Copilot CLI:

```text
copilot skill list
```

The output should include `recreate` under personal skills. In an active
Copilot session, run `/skills reload` after installing.

For Claude Code, start a new session and type:

```text
/recreate
```

Claude Code also detects changes inside an existing personal skills directory.

See [release channels and recovery](release-channels.md).

## Run

Start Edge with remote debugging enabled:

```powershell
msedge.exe --remote-debugging-port=9222 --user-data-dir="$env:TEMP\recreate-edge"
```

Capture a public site:

```powershell
npx --yes --prefer-online --registry=https://registry.npmjs.org/ recreate-cli@latest https://example.com
```

The installed package exposes both `recreate` and the temporary `site-spec`
compatibility alias:

```powershell
npx --yes recreate-cli https://example.com --out recreate-output
```

The default `implementation` profile keeps the evidence needed to rebuild and
validate the site while omitting duplicated forensic payloads. Use
`--profile full` when raw DOMSnapshot documents, complete script blobs, and
standalone component payloads are required for debugging.

Capture an authenticated tab already open in that browser:

```powershell
recreate --reuse --match example.com --out recreate-output
```

When several tabs match the same URL, pass the exact target ID from
`http://localhost:9222/json/list`:

```powershell
recreate --reuse --target <target-id> --out recreate-output
```

Ambiguous matches fail immediately instead of choosing an arbitrary stale tab.
Every CDP command also has a 30-second deadline.

Capture internal routes, panels, and other state-changing controls:

```powershell
recreate --reuse --match example.com --crawl --max-routes 30 --out recreate-output
```

Capture a safe isolated batch from exact missing-control paths. Every target is
reloaded from the home state before activation:

```powershell
recreate --reuse --target <target-id> --crawl `
  --interaction-manifest missing-interactions.json --out isolated-proof
```

Merge independently captured proofs into one complete specification:

```powershell
recreate --reuse --target <target-id> --out complete-spec `
  --merge-specs isolated-proof-1,isolated-proof-2
```

Run destructive editor probes only against a disposable or explicitly approved
editor target:

```powershell
recreate --reuse --target <target-id> --crawl --editor-probes --out recreate-output
```

Clipboard paste is a separate opt-in because it temporarily uses the system
clipboard:

```powershell
recreate --reuse --target <target-id> --crawl --editor-probes --clipboard-probe --out recreate-output
```

The prior text clipboard value and clipboard permission setting are restored
after the probe. Editor probes are disabled by default for authenticated tabs.

Explicitly allow app transitions to another path subtree on the same host:

```powershell
recreate --reuse --match example.com --crawl --allow-cross-scope --out recreate-output
```

Set explicit viewports:

```powershell
recreate --url https://example.com --out recreate-output --viewports 1440x900,390x844
```

Capture screenshots only when debugging a structured-data gap:

```powershell
recreate --url https://example.com --out recreate-output --screenshots
```

## Output

- `implementation.json`: concise agent-facing build plan and evidence index
- `state-index.json`: compact index of captured states and their detailed evidence
- `page-globals.json`: readable page-level text, geometry, styles, and controls
- `acceptance-matrix.json`: required desktop/mobile state, interaction, animation, asset, and component cells
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
recreate --reuse --target <target-id> --crawl --tooltip-probes --out recreate-output
```

General hover motion can be captured by semantic label:

```powershell
recreate --reuse --target <target-id> --crawl --hover-probes --hover-match "My Notebook" --max-routes 1 --out recreate-output
```

The hover state includes authored matching `:hover` rules, pseudo-element
keyframes and timing, forced settled geometry/styles, reduced-motion identity,
and verified pointer-leave restoration.

To prioritize one bounded control during generic crawl:

```powershell
recreate --reuse --target <target-id> --crawl --interaction-match "Add and manage sources" --max-routes 1 --out recreate-output
```

Exact semantic labels outrank substring matches and the normal generic
interaction ranking.

Public Figma Community URLs are detected automatically:

```powershell
recreate --url "https://www.figma.com/community/file/<id>/<name>" --out figma-spec
```

The extractor streams and decodes Figma's `fig-kiwi` scene graph rather than
describing the editor canvas. `implementation.json` indexes visible pages,
top-level component sections, variables, exact vector geometry, styles, and
prototype interactions. Original raster paint hashes are localized into
`snapshot-assets/`; signed Figma URLs are excluded from text artifacts.
`--profile full` also preserves the hidden backing canvas and raw
`canvas.fig`. Authenticated `/design`, `/file`, and `/proto` links discover
their binary canvas and image endpoints from CDP network events; open the file
in the remote-debug browser and pass `--reuse --match <file-key>`.

Virtualized listboxes using `aria-activedescendant` can be probed explicitly:

```powershell
recreate --reuse --target <target-id> --crawl --virtual-list-probes --out recreate-output
```

The resulting state records logical item count, mounted rows before and after
keyboard navigation, active-descendant identity, scroll-window replacement,
focus, and exact visible option geometry without inventing unmounted DOM.

Pointer-driven reorder surfaces can be probed explicitly:

```powershell
recreate --reuse --target <target-id> --crawl --drag-probes --max-routes 2 --out recreate-output
```

The probe records source and target rectangles, pointer trajectory, dragging
and drop-target attributes, intermediate geometry, final order, and verified
reverse-drag restoration.

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

Disposable WebGL targets can opt into controlled animation and input probing:

```powershell
recreate --reuse --target <target-id> --crawl --webgl-probes --max-routes 1 --out recreate-output
```

The page is reloaded with a temporary requestAnimationFrame controller, paused,
stepped by a fixed phase, pointer-dragged, reverse-dragged, numerically sampled,
then reloaded without instrumentation. Phase, interaction, and restoration
hashes and RGBA-grid distances are retained; the probe fails if the phase does
not change or the interaction has no effect. Reverse-drag restoration is
reported as a distance and exactness flag, then the page is reloaded cleanly;
nonlinear camera controls are not falsely required to invert exactly.

Same-origin iframe interactions can be probed explicitly:

```powershell
recreate --reuse --target <target-id> --crawl --iframe-probes --max-routes 1 --out recreate-output
```

The probe records every frame boundary, but reads child DOM only for a
same-origin frame. It captures child text, focus, global rectangles, computed
styles, parent `postMessage` effects, and restored child state. Cross-origin
frames retain only their URL, title, sandbox, accessibility boundary, and rect.

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

## Agent implementation artifacts

`implementation.json` is the small entrypoint. It links to:

- `acceptance-matrix.json` for every required state, viewport, interaction, and component
- `component-map.json` for searchable component identities
- `components/*.json` for readable hierarchy, text, geometry, styles, controls, assets, and responsive evidence
- `pages/`, `stylesheets/`, and `evidence/` only when exact lower-level facts are needed

These files are evidence, not shipping code. Native delivery cannot pass
`validate-native-implementation.mjs` without a structured acceptance report
covering all required states and interactions. Pass the captured reference and
native state directly so the validator computes identity, geometry, and paint
instead of trusting hand-authored comparison totals:

```powershell
node src/validate-native-implementation.mjs `
  --root <implementation-root> `
  --paths <destination-source-roots> `
  --require @1js/bebop-icons,@1js/fluentui-modern `
  --matrix <recreate-output\acceptance-matrix.json> `
  --report <structured-acceptance-report.json> `
  --comparisons <native-comparisons.json>
```

The comparison manifest must contain one entry for every state ID in the
acceptance matrix:

```json
{
  "states": [
    {
      "id": "state--1-1440x900@1",
      "reference": "recreate-output/evidence/state-home.json",
      "candidate": "native-capture/evidence/state-home.json",
      "tolerancePx": 1
    },
    {
      "id": "state--1-390x844@1",
      "reference": "recreate-output/evidence/state-home-390x844.json",
      "candidate": "native-capture/evidence/state-home-390x844.json",
      "tolerancePx": 1
    }
  ]
}
```

## Build an interactive static mock

```powershell
node build-static.mjs --spec recreate-output --out recreate-build
node recreate-build/server.mjs 4317
```

The generated build maps captured controls to local route and panel states,
preserves browser back navigation, loads captured per-state CSS, and excludes
captured application scripts. It is an **oracle only**. Never copy it into the
destination repository, link or redirect the destination app to it, or present
it as a native implementation.

Captured output can contain source code and user-visible data. Keep it out of version control unless you have permission to publish it.

## Generate readable React source

Generate a standalone React/TypeScript project for implementation agents:

```powershell
node build-react.mjs --spec recreate-output --out generated-react
cd generated-react
npm install
npm run build
```

The generator emits small typed TSX components, extracts inline SVGs into
content-addressed assets, localizes snapshot assets, and converts repeated
structures into shared components with props. CSS is deduplicated, pruned to
used selectors, and split into component-owned files plus one shared global
file while preserving source cascade order. It never emits captured
application scripts, `dangerouslySetInnerHTML`, the static reconstruction
runtime, or JSON blobs in the React source tree. Structured JSON remains
internal validation evidence.

## Measured fixture

| Metric | Result |
| --- | ---: |
| Two-viewport extraction | 27.158 s |
| Opening animation duration | 3.4 s |
| Opening animation definitions | 41 |
| Startup tracks | 374 |
| Smooth-scroll range | 32,242 px |

The reconstruction proof booted from the captured initial HTML, completed the opening sequence naturally, initialized Lenis, and changed GSAP-controlled transforms during deep scrolling.
