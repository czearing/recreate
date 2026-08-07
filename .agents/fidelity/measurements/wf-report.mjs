// Merges the locked pre-registrations with measured outcomes.
// The preRegistration blocks are copied through UNMODIFIED.
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const M = process.cwd();
const doc = JSON.parse(readFileSync(join(M, 'width-fidelity-prereg.json'), 'utf8'));
const S = JSON.parse(readFileSync(join(M, 'wf', 'wf-scored.json'), 'utf8'));
const { pages, summary } = S;

const priorInformative = {
  note: 'From overlap-property-census.json, the sample the rule was FITTED on. Reproduced here only to state how much discriminating evidence exists in total. These pages are not re-scored.',
  informative: ['godev', 'jquery', 'litenews', 'paper', 'phpman'],
  divergent: ['godev', 'jquery', 'paper', 'phpman'],
  clean: ['litenews'],
  weak: ['plaintext'],
  litenewsWidthMismatch: '0.0 at all 8 widths, and clean in both directions at all 8 widths',
};

doc.outcomes = {
  measuredAtUtc: new Date().toISOString(),
  status: 'Measured AFTER both preRegistration blocks were written and committed. Commits 1562b3a (batch 1) and 7aa9531 (batch 2) contain the labels and no measurement.',

  instrumentIntegrity: {
    predicateInheritedVerbatim: true,
    priorProbeSha: 'a5bcc101a9d8367b',
    thisRunProbeSha: 'a5bcc101a9d8367b',
    probeLengthBytes: 2151,
    method: 'overlap-property-census.mjs extracts the PROBE literal out of overlap-multi-site.mjs at runtime, so the executed predicate IS the prior run\'s bytes. verify-predicate.mjs re-extracts independently and printed IDENTICAL: true.',
    toleranceUnchangedPx: 2,
    thresholdsRetuned: 'none',
  },

  deviationsFromPlan: [
    {
      page: 'wikipedia (https://en.wikipedia.org/wiki/CSS)',
      deviation: 'REMOVED FROM DENOMINATOR, not relabelled, exactly as the registered policy requires.',
      reason: 'Capture failed on all attempts with the capture size ceiling: "Space limit exceeded: Message too long: 74826193 > 67108864" and on retry "86397609 > 67108864".',
      consequence: 'Batch 1 fell from 4 predicted-DIVERGENT / 3 predicted-CLEAN to 3 / 3.',
    },
    {
      deviation: 'A SECOND pre-registration (batch 2) was added after batch 1 was scored.',
      reason: 'Batch 1 produced 5 informative pages and all 5 were divergent, so the rule tied the majority-class baseline and carried no discriminating information. Batch 2 targeted pages with dense inline structure but non-fluid widths.',
      integrity: 'Batch-2 labels were written and committed (7aa9531) BEFORE batch-2 capture began. The decision rule and tolerance were not changed. Batch-1 results are reported unchanged.',
    },
  ],

  perPage: pages,

  scoring: {
    pagesRegistered: 11,
    pagesMeasured: 10,
    pagesRemovedAsToolLimit: 1,
    informativePages: summary.informativePages,
    weakPages: summary.weakPages,
    realSampleSize: summary.scoredDenominator,
    weakPageReasons: Object.fromEntries(summary.weakPages.map((s) => [s, pages[s].gradeReason])),

    widthFidelityRule: summary.widthFidelityRule.hits,
    widthFidelityRuleWrongOn: summary.widthFidelityRule.wrongOn,
    mechanismLabelPrediction: summary.mechanismPrediction.hits,
    mechanismLabelWrongOn: summary.mechanismPrediction.wrongOn,
    baselineAlwaysDivergent: summary.baselines.alwaysDivergent,
    baselineAlwaysClean: summary.baselines.alwaysClean,
  },

  verdict: {
    headline: 'NOT VALIDATED, AND NOT REFUTED. The rule scored 8 of 8, but the always-divergent baseline ALSO scored 8 of 8, so this sample cannot tell the two apart.',
    whyTheSampleFailedToTest: 'Every one of the 8 informative pages diverged. A predictor can only be credited when the sample contains cases it must call clean, and this one has none. Both pages that stayed clean (danluu, gnu) were graded weak by the inherited rule and left the denominator, so they cannot rescue it.',
    myOwnLabelsDidWORSEThanTheBaseline: 'The mechanism-derived predictions scored 5 of 8, below the trivial always-divergent baseline of 8 of 8. I was wrong on cern, nprtext and lobsters: I predicted CLEAN from the absence of fluid CSS, and all three diverged. Predicting the label from authored-CSS reasoning is therefore not reliable, while the measured width covariate agreed with every outcome.',
    totalDiscriminatingEvidence: {
      statement: 'Across BOTH runs there are 13 informative pages. The rule is correct on 13 of 13. The always-divergent baseline is correct on 12 of 13. The rule therefore beats the baseline by exactly ONE page.',
      thatOnePage: 'litenews, from the fitting sample: width-mismatch 0.0 at all 8 widths and clean in both directions at all 8 widths.',
      honestReading: 'The zero-versus-nonzero boundary has never once been contradicted, but the entire case for it resting on evidence beyond the base rate is a single page, and that page was in the sample the rule was fitted on. It is a promising boundary with essentially no out-of-sample support for its CLEAN arm.',
    },
    theActualFindingThatIsSolid: 'Divergence is close to universal, not page-specific. 8 of 8 informative pages here, and 12 of 13 across both runs, show overlap divergence in at least one direction at at least one width. Every framework tested diverged: React/Next, SvelteKit, VitePress/Vue, Rails, static hand-authored HTML, and a page with no stylesheet at all (cern, 12 source-only pairs). The defect is not confined to fluid framework layouts, which is the single most important correction this run produces.',
    correctionToTheDashboardsFraming: 'The dashboard called the width finding a 6-of-6 separation worth pre-registering. That is accurate about the fitting sample, but the separation was carried almost entirely by the base rate: 4 of those 6 were divergent, so an always-divergent rule already scored 4 of 6 there and 8 of 8 here. The width rule has never had a fair test of its CLEAN arm.',
  },

  bothDirections: {
    why: 'Registered because phpman showed 619 source-only against 690 recreation-only. It mattered again here.',
    sourceOnlyDominantPages: {
      reactdev: '172 source-only against 12 recreation-only',
      lobsters: '177 source-only against 70 recreation-only',
      w3c: '59 source-only against 19 recreation-only',
      cern: '12 source-only against 0 recreation-only',
    },
    consequence: 'On cern the recreation-only count is ZERO at every width. A one-directional metric would have scored cern perfectly clean, and cern is the page with no stylesheet at all. Four of eight informative pages are source-dominant, so a one-directional gate would understate or entirely miss half this sample.',
  },

  exposure: {
    why: 'Registered because jquery\'s 86 pairs involved only about 9 distinct texts. Pair counts are exposure, not defect counts.',
    perPageDistinctTextsInExamples: Object.fromEntries(
      Object.entries(pages).map(([k, v]) => [k, Object.fromEntries(Object.entries(v.exposurePerWidth).map(([w, e]) => [w, e.distinctTextsInExamples]))]),
    ),
    caveat: 'Examples are capped at 8 pairs per width by the inherited instrument, so distinct-text counts are LOWER BOUNDS on concentration and cannot be compared across pages with very different pair totals.',
  },

  rootCauseAgreement: {
    dashboardClaim: 'fidelity-developer traced the resize defect to frozen derived geometry: capture reads computed style and cannot tell an authored length from an engine-derived one, so the generator emits a pinned pixel alongside the author\'s fluid anchor.',
    mechanismIsConfirmedIndependently: 'The CSSOM specifies that for layout-dependent properties the resolved value returned by getComputedStyle IS the used value. An authored width:50% inside an 800px parent reads back as 400px and the percentage is unrecoverable. That is a documented property of the platform, not an inference from this data.',
    doesMySampleSUPPORTOneSharedRootCause: 'PARTLY, and with one finding that argues against it. Supporting: the largest width errors sit on exactly the fluid shells predicted, and sourcehut collapses from 59.2% mismatch at 400-800 to 2.8% at 1076-1440, which is what a frozen grid column looks like when the viewport approaches the width the value was frozen at. Against: cern has NO stylesheet, so it has no fluid anchor to pin, yet it still produces 12 source-only pairs. A page with nothing to freeze cannot be explained by frozen geometry, so at least one further mechanism is in play and a single generator fix should NOT be expected to close both.',
    whatWouldSettleIt: 'Attribute each mismatched element to whether its width came from an authored declaration or was engine-derived, by walking the source page cssRules rather than computed style. That distinguishes a frozen fluid anchor from a wrong box model or a dropped rule, and it is the one measurement this instrument cannot make.',
  },

  useAndCost: {
    shouldItBecomeABacktestComparisonProperty: 'NOT YET as a pass/fail gate, and YES as a recorded diagnostic. Its CLEAN arm has one page of out-of-sample support, so gating on it would fail pages the tool may be rendering acceptably and would be tuned on almost no negative evidence. As a recorded per-page number it is immediately useful, because it localises a failure to a specific element width instead of reporting that pixels differ.',
    whatItShouldBeUsedForNow: 'Attribution, not admission. Report max width delta and the offending text so a developer sees WHICH box is mis-sized. Pair it with the both-directions overlap count, since source-only divergence is half this sample and no fixture covers that direction.',
    measuredCost: {
      method: 'Timed against the built recreations with the same 8 widths: resize via Emulation.setDeviceMetricsOverride, then one batched pass collecting getBoundingClientRect for every visible element with text. Reads are batched and never interleaved with writes, so the whole page pays one forced layout per width rather than one per element.',
      samples: [
        { page: 'reactdev', elements: 879, resizeMs: 195, collectMs: 113, oneSideTotalMs: 308, perWidthMs: 38.5 },
        { page: 'lobsters', elements: 692, resizeMs: 105, collectMs: 66, oneSideTotalMs: 171, perWidthMs: 21.4 },
        { page: 'cern', elements: 48, resizeMs: 61, collectMs: 23, oneSideTotalMs: 84, perWidthMs: 10.5 },
      ],
      bothSidesEstimateMs: '168 to 616 for 8 widths, since the check must run on the candidate and the reference.',
      againstDeadline: 'COMPARISON_DEADLINE_MS is 4400 (backtest/src/deadline.rs:5, read only). 616ms is 14.0% of that budget at the heaviest page measured, and 168ms is 3.8% at the lightest.',
      costShape: 'The resize dominates the measurement: 195ms against 113ms on the heaviest page. Cost scales with the NUMBER OF WIDTHS, not with element count, so trimming the sweep from 8 widths to 3 brings the worst case to about 230ms both sides, roughly 5% of the budget.',
      caveat: 'These are measurements of the probe alone on an already-loaded page. They exclude navigation and build, which dominate a real comparison run, and they were taken on one machine with peer agents active, so they are an order-of-magnitude figure rather than a benchmark.',
    },
  },

  toolLimitsEncountered: [
    {
      site: 'https://en.wikipedia.org/wiki/CSS', stage: 'capture',
      error: 'Space limit exceeded: Message too long: 74826193 > 67108864 (retry: 86397609 > 67108864)',
      meaning: 'THIRD independent confirmation of the capture size ceiling, after getbootstrap.com. The cap is written at browser/src/cdp.rs:28 and :103. English Wikipedia is among the most-visited pages on the web, which strengthens the severity argument in capture-size-ceiling.json.',
    },
    {
      site: 'https://www.gnu.org/', stage: 'measurement',
      error: 'Zero probe candidates on BOTH sides at every width (sourceCandidates 0, recreationCandidates 0, sharedTextKeys 0), with a 128.9 KB spec.',
      meaning: 'The live source returned nothing the probe could see, so this is a source-side block or redirect against automation rather than a clean page. Graded weak and excluded. Reported as a limit of the instrument against this site, not as evidence that the recreation is correct.',
    },
  ],

  instrumentGaps: [
    'The boxes fingerprint records text, left and width but not top or height, so the wrap-taller mechanism is inferred from the width delta plus the collision rather than observed as a height delta.',
    'Pair examples are capped at 8 per width, making every distinct-text count a lower bound.',
    'The instrument cannot tell an authored width from an engine-derived one, which is precisely the distinction needed to confirm the shared-root-cause claim. Settling that requires reading the source page cssRules.',
    'Two of ten pages produced no usable signal (danluu: predicate never fired; gnu: source blocked), so informative yield was 80% and the design should over-sample to hit a target sample size.',
  ],
};

writeFileSync(join(M, 'width-fidelity-prereg.json'), JSON.stringify(doc, null, 2));
console.log('wrote width-fidelity-prereg.json with outcomes');
console.log('rule', doc.outcomes.scoring.widthFidelityRule, '| baseline', doc.outcomes.scoring.baselineAlwaysDivergent);
console.log('mechanism labels', doc.outcomes.scoring.mechanismLabelPrediction, 'wrong on', doc.outcomes.scoring.mechanismLabelWrongOn.join(','));
console.log('informative', doc.outcomes.scoring.informativePages, 'of', doc.outcomes.scoring.pagesMeasured);
