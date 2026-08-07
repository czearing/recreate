# Feature census

What real sites actually use, measured — not what someone imagined a fixture should test.

## Files

| file | contents |
|---|---|
| `census.md` / `census.json` | per-feature site counts, per-site detail, breakpoint distribution, coupled pairs, instrument gaps |
| `uncovered.md` / `uncovered.json` | features used by >= 2 sites with no fixture, sorted by descending site count |
| `raw/<site>.json` | one raw measurement record per site |
| `survey.mjs` | the instrument: launches headless Chromium over CDP and measures each site |
| `aggregate.mjs` | combines raw records into the census and diffs against the fixture corpus |

## Reproducing

```
node survey.mjs      # ~7 min, writes raw/
node aggregate.mjs   # writes census.* and uncovered.*
```

## How it measures

Rendered result, not stylesheet source. A site can ship CSS it never applies, so
every layout number comes from `getComputedStyle` on live elements.

One pre-document shim wraps `IntersectionObserver`, `ResizeObserver`,
`MutationObserver`, `HTMLElement.focus`, `HTMLDialogElement.showModal`,
`attachShadow`, `addEventListener`, `history.pushState`, `fetch` and overflow
style writes into counters, so runtime behavior is counted as it happens. One
post-load census script then reads computed styles, ARIA/DOM state and those
counters. Each page gets a scroll pass to the bottom and back to provoke
deferred work before measurement.

Media queries come from raw stylesheet text via `CSS.getStyleSheetText`, because
page-context `document.styleSheets` throws on cross-origin sheets and would
silently under-report breakpoints. Only text inside `@media`/`@container`
conditions is parsed; scanning whole stylesheets would misread ordinary
`min-width` declarations as breakpoints.

Fixture coverage is decided by scanning each fixture's control markup for the
feature, never by matching directory names.

## What it cannot see

`optimistic-update` has no DOM signature and is reported as undetected on all
sites, not as absent. `focus-trapping` and `scroll-locking` only exist while a
modal is open, so they are proxies. `virtualized-list` is found only through
known library class names or `aria-setsize`, so a hand-rolled virtualizer reads
as absent. These are limits of the instrument and are listed in the census under
instrument gaps.

## Scope

This directory produces the census only. It does not rank priority, author
fixtures, or touch the comparator or generator.
