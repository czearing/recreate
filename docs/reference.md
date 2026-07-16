# Recreate reference

## MVP scope

Recreate captures one rendered page at a time. It does not crawl routes or
subsites.

The capture includes:

- narrow mobile, mobile, tablet, desktop, and wide desktop DOM structure
- computed layout and visual styles
- authored CSS rules and local font assets
- images, SVGs, data assets, and browser blob assets
- lifecycle and Web Animations keyframes
- bounded in-page controls that reveal menus, dialogs, toggles, or overlays
- exact text-node order and geometry

## Commands

```text
recreate capture <url>
recreate capture --reuse --target <target-id>
recreate open <url>
recreate generate --spec <spec.json> --out <directory>
recreate verify --spec <spec.json> --url <generated-url>
recreate verify --spec <spec.json> --url <generated-url> --interaction 1
recreate install [--copilot | --claude | --all]
recreate skill
```

Capture options:

```text
--cdp-url http://127.0.0.1:9222
--out recreate-output
--viewports 1920x1080,1440x900,768x1024,390x844,320x568
```

## Browser access

The skill uses `recreate open` to create the source in a dedicated visible
Chromium profile and inspects the returned target before capture.

If the visible page could be either the intended design or an access step, the
skill asks one short question. Credentials and session state remain inside the
browser. Capture reuses the exact CDP endpoint and target identifier.
Reuse capture does not navigate or reload the authenticated target.

## Generated React

The generated project contains:

```text
react/
  public/assets/
  src/components/<Component>/<Component>.jsx
  src/components/index.js
  src/App.jsx
  src/states.jsx
  src/styles.css
```

Repeated structures and exact style matrices are deduplicated. Responsive
classes use non-overlapping viewport bands so adjacent layouts cannot collide.

## Validation

`acceptance.json` checks evidence completeness. `recreate verify` loads the
generated page through CDP and compares text, visible geometry, and key computed
styles for every captured node and interaction state.

Verification runs against the production build after React conversion,
component deduplication, responsive compaction, and Vite minification. It
requires identical expected and actual node-path sets, preserved structure,
behavior-bearing attributes, pseudo content, text, geometry, and key styles.
Missing or unexpected nodes fail verification.

CI and publish validation run generic browser fixtures through a release gate.
The retained platform evidence artifacts record built generated-project parity at
1920x1080, 1440x900, 768x1024, 390x844, and 320x568; horizontal overflow;
browser console and network errors; capture, build, and browser runtime;
generated source and build size; keyboard activation and focus restoration; and
reduced-motion behavior.

## Authenticated reference proof

Authenticated captures are validated from persisted specifications so release
proof does not depend on an expiring browser session. The final MVP reference
checks baseline and interaction parity, embedded authenticated assets, five
responsive layouts, normal motion, reduced motion, keyboard activation, focus
restoration, console errors, and network errors.
