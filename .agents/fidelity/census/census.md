# Real-site feature census

Generated 2026-08-07T06:50:06.615Z - 18/18 sites, 16 frameworks.

Instrument: headless Chromium over CDP at 1440x900. One pre-document shim wraps IntersectionObserver, ResizeObserver, MutationObserver, HTMLElement.focus, dialog.showModal, attachShadow, addEventListener, history.pushState, fetch and overflow writes into counters. One post-load census script reads getComputedStyle and ARIA/DOM state. Media queries come from raw stylesheet text via CSS.getStyleSheetText, so cross-origin sheets are not silently dropped. Every number below is a measurement, not an estimate.

## Sites surveyed

| site | framework | why in scope | elements | depth | breakpoints | sheets read |
|---|---|---|---|---|---|---|
| nextjs | Next.js (React) | Reference implementation of the dominant React meta-framework; App Router RSC output. | 2274 | 19 | 34 | 10/10 |
| tailwindcss | Next.js (React) | Utility-CSS site: densest source of declared breakpoints and container queries. | 2600 | 22 | 17 | 2/2 |
| vercel | Next.js (React) | Commercial app shell by the framework authors; heavy motion and deferred load. | 1112 | 25 | 48 | 10/10 |
| stripe | React | Industry benchmark for high-fidelity marketing layout and custom interaction surfaces. | 2847 | 32 | 28 | 5/5 |
| github | React + Primer (Rails hybrid) | Large production app: menus, dialogs, tooltips, virtualization, partial hydration. | 1772 | 25 | 43 | 34/34 |
| vuejs | Vue 3 / VitePress | Vue ecosystem reference; SSG plus client hydration. | 798 | 19 | 19 | 3/3 |
| nuxt | Nuxt 3 (Vue) | Vue meta-framework with island hydration and route-level deferred load. | 1973 | 25 | 7 | 21/21 |
| angular | Angular | Angular reference site; CDK-driven overlay and a11y interaction surfaces. | 477 | 22 | 12 | 19/19 |
| svelte | SvelteKit | Compiler-based framework with no virtual DOM; different runtime signature. | 396 | 12 | 15 | 7/7 |
| solidjs | SolidJS | Fine-grained reactivity framework; small-team CSS conventions. | 388 | 21 | 5 | 2/2 |
| astro | Astro | Islands architecture: mostly static HTML with selectively hydrated widgets. | 1831 | 19 | 8 | 9/9 |
| reactrouter | React Router / Remix | Data-router framework whose defining feature is optimistic UI and deferred data. | 123 | 9 | 7 | 2/2 |
| emberjs | Ember | Older-generation MVC framework; legacy CSS idioms (float, absolute) survive here. | 554 | 12 | 9 | 2/2 |
| mdn | Yari (React SSR + vanilla) | Long-form document page: multi-column, sticky sidebar, deep DOM. | 3656 | 23 | 12 | 129/129 |
| wikipedia | MediaWiki (vanilla JS) | No modern framework: float-based infoboxes, table layout, legacy CSS. | 4138 | 31 | 7 | 18/19 |
| hackernews | None (static HTML) | Table-layout control case; proves the instrument reports absence, not noise. | 821 | 15 | 21 | 1/1 |
| bbc | React (SSR) | High-traffic news grid: carousels, lazy media, many breakpoints. | 1448 | 22 | 14 | 8/10 |
| shopify | Remix (React) | Large commerce marketing site; independent of the framework-docs monoculture. | 2264 | 17 | 31 | 3/3 |

## Feature census

| feature | group | sites | share | detection | fixture coverage |
|---|---|---|---|---|---|
| media-query | responsive | 18 | 100% | stylesheet-text | responsive-isolation, responsive-range |
| absolute | layout | 17 | 94% | computed | layout-isolation, raster-attribution |
| deferred-load | behavior | 17 | 94% | rendered + runtime (IntersectionObserver wrap) | **none** |
| flex | layout | 17 | 94% | computed | raster-attribution, responsive-range, semantic-layout |
| grid | layout | 17 | 94% | computed | responsive-range, semantic-layout |
| border-radius | layout | 16 | 89% | computed | focus-behavior, raster-attribution, semantic-layout |
| css-custom-property | layout | 16 | 89% | computed | **none** |
| svg | interaction | 16 | 89% | dom | visual-assets |
| transform | layout | 16 | 89% | computed | animation-timing, motion-behavior, motion-isolation |
| css-gradient | layout | 15 | 83% | computed | raster-attribution |
| disclosure-button | interaction | 15 | 83% | aria/dom | **none** |
| webfont | layout | 15 | 83% | computed | **none** |
| z-index-stacking | layout | 15 | 83% | computed | **none** |
| aspect-ratio | layout | 14 | 78% | computed | **none** |
| box-shadow | layout | 14 | 78% | computed | **none** |
| focus-restoration | behavior | 14 | 78% | runtime (wrapped HTMLElement.focus) + proxy | focus-behavior |
| clip-path | layout | 13 | 72% | computed | **none** |
| css-transition | layout | 13 | 72% | computed | motion-behavior |
| fixed | layout | 12 | 67% | computed | layout-isolation |
| responsive-image | interaction | 11 | 61% | dom | **none** |
| sticky | layout | 11 | 61% | computed | **none** |
| custom-element | interaction | 10 | 56% | dom | **none** |
| drag-listener-registrations | interaction | 10 | 56% | runtime (inflated by framework event delegation - see gaps) | **none** |
| live-region | interaction | 10 | 56% | aria/dom | accessibility-isolation, async-timer |
| css-animation | layout | 9 | 50% | computed | animation-timing, motion-isolation |
| accordion | interaction | 8 | 44% | aria/dom | **none** |
| menu | interaction | 8 | 44% | aria/dom | **none** |
| tab-set | interaction | 8 | 44% | aria/dom | **none** |
| backdrop-filter | layout | 7 | 39% | computed | **none** |
| form | interaction | 7 | 39% | dom | **none** |
| skip-link | interaction | 7 | 39% | dom | **none** |
| container-query | layout | 5 | 28% | computed | **none** |
| iframe | interaction | 5 | 28% | dom | **none** |
| scroll-container | layout | 5 | 28% | computed | **none** |
| shadow-dom | interaction | 5 | 28% | dom+runtime | **none** |
| tooltip | interaction | 5 | 28% | aria/dom | **none** |
| focus-trapping | behavior | 4 | 22% | proxy (modal surface present; trap only observable while open) | focus-behavior |
| table-layout | layout | 4 | 22% | computed | **none** |
| video | interaction | 4 | 22% | dom | **none** |
| canvas | interaction | 3 | 17% | dom | raster-attribution |
| carousel | interaction | 3 | 17% | aria/dom | **none** |
| float | layout | 3 | 17% | computed | **none** |
| multi-column | layout | 3 | 17% | computed | **none** |
| scroll-locking | behavior | 3 | 17% | proxy (css lock rule / runtime overflow write) | **none** |
| scroll-snap | layout | 3 | 17% | computed | **none** |
| dialog | interaction | 2 | 11% | aria/dom | focus-behavior |
| combobox | interaction | 1 | 6% | aria/dom | **none** |
| drag-target | interaction | 0 | 0% | aria/dom | **none** |
| optimistic-update | behavior | 0 | 0% | UNDETECTED - no DOM signature | **none** |
| subgrid | layout | 0 | 0% | computed | **none** |
| virtualized-list | interaction | 0 | 0% | aria/dom | **none** |

## Declared media query breakpoint widths

| width (px) | sites declaring |
|---|---|
| 640 | 13 |
| 768 | 11 |
| 1280 | 11 |
| 480 | 9 |
| 600 | 9 |
| 1024 | 9 |
| 500 | 6 |
| 320 | 5 |
| 400 | 5 |
| 900 | 5 |
| 1200 | 5 |
| 1536 | 5 |
| 384 | 4 |
| 576 | 4 |
| 639 | 4 |
| 840 | 4 |
| 960 | 4 |
| 1080 | 4 |
| 1300 | 4 |
| 1400 | 4 |
| 1600 | 4 |
| 360 | 3 |
| 370 | 3 |
| 430 | 3 |
| 448 | 3 |
| 450 | 3 |
| 512 | 3 |
| 599 | 3 |
| 601 | 3 |
| 720 | 3 |
| 750 | 3 |
| 767 | 3 |
| 800 | 3 |
| 992 | 3 |
| 1000 | 3 |
| 401 | 2 |
| 420 | 2 |
| 540 | 2 |
| 544 | 2 |
| 618 | 2 |
| 721 | 2 |
| 769 | 2 |
| 899 | 2 |
| 961 | 2 |
| 1007 | 2 |
| 1008 | 2 |
| 1090 | 2 |
| 1100 | 2 |
| 1150 | 2 |
| 1279 | 2 |
| 1299 | 2 |
| 1439 | 2 |
| 1440 | 2 |
| 1441 | 2 |
| 1496 | 2 |
| 1500 | 2 |
| 192 | 1 |
| 242 | 1 |
| 248 | 1 |
| 252 | 1 |
| 264 | 1 |
| 300 | 1 |
| 340 | 1 |
| 375 | 1 |
| 380 | 1 |
| 383 | 1 |
| 389 | 1 |
| 390 | 1 |
| 395 | 1 |
| 399 | 1 |
| 408 | 1 |
| 410 | 1 |
| 421 | 1 |
| 426 | 1 |
| 440 | 1 |
| 460 | 1 |
| 463 | 1 |
| 470 | 1 |
| 475 | 1 |
| 479 | 1 |
| 496 | 1 |
| 499 | 1 |
| 504 | 1 |
| 509 | 1 |
| 510 | 1 |
| 520 | 1 |
| 542 | 1 |
| 543 | 1 |
| 550 | 1 |
| 560 | 1 |
| 568 | 1 |
| 650 | 1 |
| 670 | 1 |
| 680 | 1 |
| 689 | 1 |
| 690 | 1 |
| 700 | 1 |
| 701 | 1 |
| 706 | 1 |
| 712 | 1 |
| 740 | 1 |
| 759 | 1 |
| 760 | 1 |
| 775 | 1 |
| 780 | 1 |
| 794 | 1 |
| 803 | 1 |
| 804 | 1 |
| 809 | 1 |
| 810 | 1 |
| 820 | 1 |
| 831 | 1 |
| 832 | 1 |
| 844 | 1 |
| 860 | 1 |
| 875 | 1 |
| 876 | 1 |
| 877 | 1 |
| 880 | 1 |
| 882 | 1 |
| 890 | 1 |
| 891 | 1 |
| 893 | 1 |
| 896 | 1 |
| 901 | 1 |
| 939 | 1 |
| 940 | 1 |
| 959 | 1 |
| 970 | 1 |
| 980 | 1 |
| 985 | 1 |
| 991 | 1 |
| 1004 | 1 |
| 1011 | 1 |
| 1012 | 1 |
| 1020 | 1 |
| 1028 | 1 |
| 1029 | 1 |
| 1033 | 1 |
| 1036 | 1 |
| 1039 | 1 |
| 1040 | 1 |
| 1044 | 1 |
| 1051 | 1 |
| 1079 | 1 |
| 1104 | 1 |
| 1108 | 1 |
| 1115 | 1 |
| 1120 | 1 |
| 1140 | 1 |
| 1169 | 1 |
| 1170 | 1 |
| 1199 | 1 |
| 1201 | 1 |
| 1259 | 1 |
| 1260 | 1 |
| 1263 | 1 |
| 1264 | 1 |
| 1281 | 1 |
| 1295 | 1 |
| 1320 | 1 |
| 1330 | 1 |
| 1349 | 1 |
| 1350 | 1 |
| 1370 | 1 |
| 1377 | 1 |
| 1380 | 1 |
| 1399 | 1 |
| 1420 | 1 |
| 1431 | 1 |
| 1460 | 1 |
| 1599 | 1 |
| 1604 | 1 |
| 1609 | 1 |
| 1680 | 1 |
| 1728 | 1 |
| 1801 | 1 |
| 2300 | 1 |
| 2400 | 1 |
| 8192 | 1 |

## Feature pairs that co-occur across the sample

`identical-site-set` means the two features are used by exactly the same sites, so no site exercises one without the other.

| pair | sites | coupling | single fixture exercising both |
|---|---|---|---|
| absolute + deferred-load | 17 | identical-site-set | **none** |
| absolute + flex | 17 | identical-site-set | raster-attribution |
| absolute + grid | 17 | identical-site-set | **none** |
| deferred-load + flex | 17 | identical-site-set | **none** |
| deferred-load + grid | 17 | identical-site-set | **none** |
| flex + grid | 17 | identical-site-set | responsive-range |
| border-radius + css-custom-property | 16 | identical-site-set | **none** |

## Instrument gaps

| item | reason | measurement |
|---|---|---|
| focus-trapping | measured by proxy only: proxy (modal surface present; trap only observable while open) | proxySiteCount=4 |
| focus-restoration | measured by proxy only: runtime (wrapped HTMLElement.focus) + proxy | proxySiteCount=14 |
| scroll-locking | measured by proxy only: proxy (css lock rule / runtime overflow write) | proxySiteCount=3 |
| optimistic-update | UNDETECTED - no DOM signature | undetectedSites=18 |
| media-query | stylesheet text retrieval | styleSheetsRead=285 styleSheetsUnreadable=3 |
| drag-listener-registrations | runtime listener count is inflated by framework root event delegation (React registers dragstart/dragover/drop on the container regardless of any drag UI); use drag-target for the DOM signal | - |
| virtualized-list | detected only via known library class names and aria-setsize overflow; a hand-rolled virtualizer with neither would read as absent | - |
