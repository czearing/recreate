# Recreate reference

## MVP scope

Recreate captures one rendered page at a time. It does not crawl routes or
subsites.

The capture includes:

- desktop and mobile DOM structure
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
--viewports 1440x900,390x844
```

## Browser access

The skill opens the source in an installed Chromium browser and inspects the
rendered page before capture.

If the visible page could be either the intended design or an access step, the
skill asks one short question. Credentials and session state remain inside the
browser. Capture reuses the exact CDP endpoint and target identifier.

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
classes include every captured viewport so mobile overrides cannot collide.

## Validation

`acceptance.json` checks evidence completeness. `recreate verify` loads the
generated page through CDP and compares text, visible geometry, and key computed
styles for every captured node and interaction state.
