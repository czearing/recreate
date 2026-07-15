<div align="center">

# Recreate

**Turn live interfaces into verified native code.**

[![CI](https://github.com/czearing/recreate/actions/workflows/ci.yml/badge.svg)](https://github.com/czearing/recreate/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/recreate-cli)](https://www.npmjs.com/package/recreate-cli)
[![Node](https://img.shields.io/node/v/recreate-cli)](package.json)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Recreate captures structure, styles, assets, responsive rules, animation, and
behavior directly from a live browser. Coding agents use that evidence to
rebuild interfaces without guessing from screenshots.

</div>

## Quick start

```bash
npx recreate-cli https://example.com
```

Recreate writes a build-ready package to `recreate-output/`.

Give it to Claude, Copilot, or another coding agent:

```text
Use the Recreate output to implement this interface natively.
Validate every captured state before finishing.
```

## Why Recreate

- **Browser ground truth** — DOM, CSSOM, geometry, assets, and events.
- **Behavior included** — routes, menus, dialogs, focus, hover, and restoration.
- **Responsive evidence** — exact constraints across captured viewports.
- **Agent-ready React** — readable components instead of raw browser dumps.
- **Numeric validation** — geometry, styles, content, and interaction coverage.
- **No screenshot inference** — screenshots are optional diagnostics only.

## How it works

1. **Capture** — connect to the live page through Chrome DevTools Protocol.
2. **Model** — isolate components, states, assets, rules, and interactions.
3. **Prove** — emit native implementation guidance and an acceptance matrix.

## Output

| Path | Purpose |
| --- | --- |
| `react/` | Readable generated React reference |
| `implementation.json` | Concise implementation blueprint |
| `acceptance-matrix.json` | Required states, viewports, and interactions |
| `component-map.json` | Component hierarchy and provenance |
| `evidence/` | Exact geometry, styles, focus, and behavior |
| `snapshot-assets/` | Content-addressed local assets |

## Common options

```bash
# Desktop and mobile
recreate https://example.com --viewports 1440x900,390x844

# Capture routes and interactive states
recreate https://example.com --crawl --max-routes 30

# Use an already-open browser tab
recreate --reuse --match example.com

# Full forensic evidence
recreate https://example.com --profile full
```

## Requirements

- Node.js 22+
- Chrome, Edge, or another Chromium browser

## Principles

- Native implementation, never iframe delivery.
- Structured evidence before visual approximation.
- Destructive probes require explicit approval.
- Captured application scripts never ship as generated code.
- A result is incomplete while required evidence is missing.

## Documentation

- [Full reference](docs/reference.md)
- [Release channels](docs/release-channels.md)
- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)

## License

MIT
