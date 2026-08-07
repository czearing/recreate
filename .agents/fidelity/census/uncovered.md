# Uncovered feature set

Rule: feature used by >= 2 surveyed sites with no fixture whose control markup exercises it. 30 uncovered features across 18 surveyed sites.

| rank | feature | group | sites | sites using it |
|---|---|---|---|---|
| 1 | deferred-load | behavior | 17 | nextjs, tailwindcss, vercel, stripe, github, vuejs, nuxt, angular, svelte, solidjs, astro, reactrouter, emberjs, mdn, wikipedia, bbc, shopify |
| 2 | css-custom-property | layout | 16 | nextjs, tailwindcss, vercel, stripe, github, vuejs, nuxt, angular, svelte, solidjs, astro, reactrouter, emberjs, mdn, wikipedia, shopify |
| 3 | disclosure-button | interaction | 15 | tailwindcss, vercel, stripe, github, vuejs, nuxt, angular, svelte, solidjs, astro, emberjs, mdn, wikipedia, bbc, shopify |
| 4 | webfont | layout | 15 | nextjs, tailwindcss, vercel, stripe, github, vuejs, nuxt, angular, svelte, astro, reactrouter, emberjs, mdn, bbc, shopify |
| 5 | z-index-stacking | layout | 15 | nextjs, tailwindcss, vercel, stripe, github, vuejs, nuxt, angular, svelte, solidjs, astro, mdn, wikipedia, bbc, shopify |
| 6 | aspect-ratio | layout | 14 | nextjs, tailwindcss, vercel, stripe, github, nuxt, svelte, astro, reactrouter, emberjs, wikipedia, hackernews, bbc, shopify |
| 7 | box-shadow | layout | 14 | nextjs, tailwindcss, vercel, stripe, github, vuejs, nuxt, angular, solidjs, astro, emberjs, mdn, wikipedia, shopify |
| 8 | clip-path | layout | 13 | nextjs, tailwindcss, vercel, stripe, github, vuejs, nuxt, angular, svelte, astro, mdn, wikipedia, shopify |
| 9 | responsive-image | interaction | 11 | nextjs, tailwindcss, vercel, stripe, vuejs, nuxt, svelte, solidjs, wikipedia, bbc, shopify |
| 10 | sticky | layout | 11 | nextjs, tailwindcss, vercel, github, nuxt, angular, solidjs, mdn, wikipedia, bbc, shopify |
| 11 | custom-element | interaction | 10 | nextjs, tailwindcss, vercel, stripe, github, angular, svelte, astro, mdn, bbc |
| 12 | drag-listener-registrations | interaction | 10 | nextjs, tailwindcss, vercel, stripe, github, nuxt, reactrouter, mdn, bbc, shopify |
| 13 | accordion | interaction | 8 | vercel, stripe, github, solidjs, astro, mdn, wikipedia, shopify |
| 14 | menu | interaction | 8 | tailwindcss, vercel, github, vuejs, nuxt, angular, wikipedia, shopify |
| 15 | tab-set | interaction | 8 | tailwindcss, github, nuxt, angular, solidjs, astro, emberjs, shopify |
| 16 | backdrop-filter | layout | 7 | nextjs, tailwindcss, stripe, github, nuxt, angular, solidjs |
| 17 | form | interaction | 7 | nextjs, github, nuxt, svelte, astro, wikipedia, hackernews |
| 18 | skip-link | interaction | 7 | github, vuejs, svelte, astro, mdn, bbc, shopify |
| 19 | container-query | layout | 5 | tailwindcss, vercel, stripe, angular, shopify |
| 20 | iframe | interaction | 5 | stripe, vuejs, solidjs, bbc, shopify |
| 21 | scroll-container | layout | 5 | angular, solidjs, mdn, wikipedia, bbc |
| 22 | shadow-dom | interaction | 5 | nextjs, tailwindcss, vercel, github, mdn |
| 23 | tooltip | interaction | 5 | tailwindcss, stripe, github, nuxt, astro |
| 24 | table-layout | layout | 4 | nuxt, mdn, wikipedia, hackernews |
| 25 | video | interaction | 4 | github, svelte, wikipedia, shopify |
| 26 | carousel | interaction | 3 | tailwindcss, stripe, nuxt |
| 27 | float | layout | 3 | github, wikipedia, bbc |
| 28 | multi-column | layout | 3 | vuejs, wikipedia, shopify |
| 29 | scroll-locking | behavior | 3 | github, angular, wikipedia |
| 30 | scroll-snap | layout | 3 | tailwindcss, stripe, shopify |

## Uncovered feature pairs

Pairs that co-occur across the sample with no single fixture exercising both. NIST fault data shows single-factor faults account for only 20-68% of failures while 2-way interactions reach 65-97%, so isolated fixtures leave these untested.

| rank | pair | sites | coupling |
|---|---|---|---|
| 1 | absolute + deferred-load | 17 | identical-site-set |
| 2 | absolute + grid | 17 | identical-site-set |
| 3 | deferred-load + flex | 17 | identical-site-set |
| 4 | deferred-load + grid | 17 | identical-site-set |
| 5 | border-radius + css-custom-property | 16 | identical-site-set |
