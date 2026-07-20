# Behavioral Fidelity Completion Plan

This checklist is complete only when every item is implemented, tested, validated,
committed, pushed to `main`, and demonstrated in a newly generated site.

## Architecture

- [x] Add typed trigger, surface, sequence, reducer, textarea, anchor, and carousel models.
- [x] Preserve unique bindings for repeated controls while sharing equivalent surfaces.
- [x] Move routing, dismissal, focus, timing, anchoring, and carousel decisions out of generated JSX strings.
- [x] Replace runtime string patching with small generated runtime modules.
- [x] Keep production source files below 200 lines where practical; tests are exempt.

## Isolated tests

- [x] Add table-driven Rust tests for every pure function and decision branch.
- [x] Test exact, semantic, repeated, missing, stale, and reordered trigger identities.
- [x] Test open, toggle, Escape, outside-click, replacement, and focus restoration transitions.
- [x] Test irregular mutation timing, duplicate collapse, cleanup, remount, and two loops without wall-clock waits.
- [x] Test textarea setter selection, input/change/composition events, keyboard behavior, wrapping, and placeholder coexistence.
- [x] Test menu anchoring, resize, scrolling, carousel reorder, and stale trigger rejection.
- [x] Test carousel forward, reverse, boundaries, resize, transform, and disabled states.
- [x] Keep the isolated Rust and generated-runtime gate below five seconds.

## Integration

- [x] Generate the exact runtime modules tested in isolation.
- [x] Add a minimal local fixture gate with no live application, authentication, network, or Vite dependency.
- [x] Verify one mounted DOM across representative widths with continuous flex and wrapping behavior.
- [x] Verify Search, account, launcher, every repeated overflow control, prompt copy, textarea, carousel, and close restoration.
- [x] Run the existing targeted browser gate only after isolated tests pass.
- [x] Run formatting, Clippy, targeted tests, generated React build, and required broader tests.

## Release

- [x] Regenerate a new authenticated fidelity candidate.
- [x] Verify the candidate directly in the browser at representative layouts.
- [x] Confirm no token, profile, database, capture, or machine-local artifact is staged.
- [ ] Commit with required trailers and push `main`.
- [ ] Confirm the remote commit and provide the new preview URL.
