# Recreate Independent Fidelity Oracle

`recreate-oracle` records browser-authoritative evidence from a source page and
replays the immutable scenario tape against a candidate. It is a separate crate
and does not import Recreate capture, generation, or fidelity code.

Both Recreate and the oracle use the neutral `recreate-browser` workspace crate
for browser targets, executable discovery, CDP transport, events, evaluation,
and errors. This keeps bundle access identical without importing Recreate's
capture or generation interpretation into the oracle.

## Certification contract

Certification is exact inside the artifact's pinned finite contract. Every
checkpoint must match in all eight domains: structure, geometry, computed
style, accessibility, interaction state, asynchronous state, browser animation
state, and final compositor pixels. Missing checkpoints, browser errors,
unknown domains, corrupt artifacts, or environment drift fail closed.

Compositor images are hashed in memory and are never written to disk. Reports
provide an exact parts-per-million score and the shortest typed JSON path to
each differing domain. A score of 100% is not certification unless coverage is
complete and there are no blockers.

## Commands

```console
cargo run -p recreate-oracle --release -- record https://source.example \
  --out source.oracle.json --widths 320,390,768,1280,1440

cargo run -p recreate-oracle --release -- compare source.oracle.json \
  https://candidate.example --out report.json

cargo run -p recreate-oracle --release -- qualify --fixtures oracle/fixtures \
  --holdouts oracle/holdouts --out target/oracle-qualification.json

cargo run -p recreate-oracle --release -- benchmark source.oracle.json \
  https://candidate.example --iterations 20

cargo run -p recreate-oracle --release -- record https://authenticated.example \
  --out source.oracle.json --cdp-url http://127.0.0.1:9223 --target TARGET_ID
```

`record` runs source-self three times, stores ascending and descending resize
histories, adds every discovered media/container boundary at `N-1`, `N`, and
`N+1`, seeks every presented 60 Hz frame of finite animation cycles, advances a
virtual-time async boundary, and replays source-derived hover, click, and Escape
tapes. `compare` requires the same pinned Chromium product, revision, protocol,
platform, architecture, flags, locale, timezone, color scheme, motion
preference, and device scale factor used by the source recording.

Discovery executes in a pre-document trace lane. Its clean structure,
accessibility, motion, geometry, style, interaction, and decoded compositor
pixel roots must equal the uninstrumented source lane. Unsupported or unbounded
surfaces become explicit obligations and prevent artifact verification rather
than being sampled or ignored.

## Qualification and performance

The checked-in isolated page contains one-pixel responsive boundaries, delayed
virtual-time behavior, a complete finite animation cycle, semantic controls,
trusted inputs, accessibility, clipping, hit testing, complete computed style,
and decoded final compositor pixels. Qualification requires source-self
acceptance, acceptance of an independently formatted equivalent page, rejection
and correct first-domain localization of every public and separately loaded
holdout mutant, byte-equivalent
optimized/reference reports, and warm p95/p99 no slower than 2,000/2,500 ms.

The locked soak workload runs 1,000 certifications in one browser service.
Windows browsers are assigned to a kill-on-close Job Object so renderer
descendants cannot survive the oracle. The retained soak evidence records raw
latencies and verifies that browser target count does not grow.

Unit and property tests prove deterministic artifact sealing, corruption
rejection, scenario coverage, shortest counterexamples, and equality between
the optimized digest path and a slow value comparison. Architecture tests
enforce dependency isolation and the sub-200-line shipping-source limit.
