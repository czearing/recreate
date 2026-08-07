// Appends the batch-split scoring and the mechanism signature test to the
// deliverable WITHOUT touching either pre-registration block, then proves both
// blocks are byte-identical to what was committed before any capture ran.
import fs from 'node:fs';
import crypto from 'node:crypto';

const F = 'width-fidelity-prereg.json';
const d = JSON.parse(fs.readFileSync(F, 'utf8'));
const sha = o => crypto.createHash('sha256').update(JSON.stringify(o)).digest('hex').slice(0, 16);
const before = { b1: sha(d.preRegistration), b2: sha(d.preRegistrationBatch2) };

const disc = JSON.parse(fs.readFileSync('wf/wf-discontinuity.json', 'utf8'));
const sig = JSON.parse(fs.readFileSync('wf/wf-signature.json', 'utf8'));

const B1 = ['wikipedia', 'reactdev', 'sveltedev', 'vuejs', 'cern', 'danluu', 'nprtext'];
const B2 = ['lobsters', 'gnu', 'w3c', 'sourcehut'];
const outcome = {
  reactdev: 'DIVERGENT', sveltedev: 'DIVERGENT', vuejs: 'DIVERGENT', cern: 'DIVERGENT',
  danluu: 'CLEAN', nprtext: 'DIVERGENT', lobsters: 'DIVERGENT', gnu: null,
  w3c: 'DIVERGENT', sourcehut: 'DIVERGENT',
};
const grade = {
  reactdev: 'informative', sveltedev: 'informative', vuejs: 'informative', cern: 'informative',
  danluu: 'weak', nprtext: 'informative', lobsters: 'informative',
  gnu: 'weak', w3c: 'informative', sourcehut: 'informative',
  wikipedia: 'excluded',
};
const pred = { ...d.preRegistration.predictedLabels, ...d.preRegistrationBatch2.predictedLabels };

function scoreBatch(names) {
  const inf = names.filter(n => grade[n] === 'informative');
  const div = inf.filter(n => outcome[n] === 'DIVERGENT').length;
  const clean = inf.length - div;
  const majority = div >= clean ? 'DIVERGENT' : 'CLEAN';
  return {
    pagesRegistered: names.length,
    pagesExcludedAsToolLimit: names.filter(n => grade[n] === 'excluded'),
    pagesGradedWeak: names.filter(n => grade[n] === 'weak'),
    informativePages: inf,
    informativeCount: inf.length,
    outcomeClassBalance: { DIVERGENT: div, CLEAN: clean },
    ruleScore: `${inf.length} informative, rule correct on ${inf.length}`,
    ruleCorrect: inf.length,
    majorityClassBaseline: majority,
    baselineCorrect: majority === 'DIVERGENT' ? div : clean,
    mechanismLabelCorrect: inf.filter(n => pred[n].predicted === outcome[n]).length,
    mechanismLabelWrongOn: inf.filter(n => pred[n].predicted !== outcome[n]),
    verdict: null,
  };
}

const b1 = scoreBatch(B1), b2 = scoreBatch(B2), pooled = scoreBatch([...B1, ...B2]);
b1.verdict = `NULL. Rule ${b1.ruleCorrect}/${b1.informativeCount}, always-DIVERGENT baseline ${b1.baselineCorrect}/${b1.informativeCount}. Tie. Every informative page fell in one class, so the sample cannot discriminate.`;
b2.verdict = `NULL, and for the exact reason the dashboard named in advance. Batch 2 was enriched 2-of-4 for predicted-CLEAN, but the enrichment FAILED to produce a single informative CLEAN page: lobsters was predicted CLEAN and measured DIVERGENT, gnu returned zero probe candidates on both sides and is weak. Rule ${b2.ruleCorrect}/${b2.informativeCount}, baseline ${b2.baselineCorrect}/${b2.informativeCount}. Tie again.`;
pooled.verdict = `Reported for completeness ONLY and must not be quoted alone. Rule ${pooled.ruleCorrect}/${pooled.informativeCount}, baseline ${pooled.baselineCorrect}/${pooled.informativeCount}. Pooling an enriched batch with an unenriched one measures the sampling design, not the rule; both strata are individually null so the pooled figure carries no information either.`;

const powered = Object.entries(disc).filter(([, v]) => v.testable && v.poweredForStepTest);
const rawOK = powered.filter(([, v]) => v.largestStepIsAtSampledBoundary).map(([s]) => s);
const normOK = powered.filter(([, v]) => v.largestNormStepIsAtSampledBoundary).map(([s]) => s);

d.batchSplitAndMechanismTest = {
  writtenAtUtc: new Date().toISOString(),
  correctionToTheTaskAsGiven: {
    claim: 'The task asks for constraints "on batch 2" as though batch 2 were still to be run.',
    finding: 'Batch 2 was already registered, captured, measured and committed in the previous turn. Its labels are in preRegistrationBatch2 (commit 7aa9531, before capture) and its results in outcomes (commit 113bd56).',
    consequence: 'No new pages were captured this turn, which also satisfies the instruction not to seek a third batch with easier pages. This turn re-reports the existing data under the required batch split and adds the mechanism test that had not been done.',
  },
  requirement1_batchesReportedSeparatelyWithBaseline: { batch1: b1, batch2: b2, pooled },
  requirement2_decisiveCell: {
    definition: 'A page predicted CLEAN that MEASURES CLEAN and on which the overlap predicate actually fired.',
    occurred: false,
    candidatesAndWhyEachFailed: {
      danluu: 'batch 1, predicted CLEAN, measured CLEAN - but the predicate never fired (0 candidate pairs on both sides at every width). Weak. Not evidence for the rule.',
      cern: 'batch 1, predicted CLEAN, measured DIVERGENT (12 source-only pairs). Prediction wrong.',
      nprtext: 'batch 1, predicted CLEAN, measured DIVERGENT. Prediction wrong.',
      lobsters: 'batch 2, predicted CLEAN, measured DIVERGENT (70 recreation-only, 177 source-only). Prediction wrong.',
      gnu: 'batch 2, predicted CLEAN, zero probe candidates on BOTH sides - a source-side block, not a clean recreation. Weak.',
    },
    conclusion: 'Across 10 registered pages and 5 deliberate predicted-CLEAN choices, the decisive cell never occurred once. The CLEAN arm of the width rule remains entirely untested out of sample. Per the standing instruction, no third batch was sought.',
  },
  requirement3_mechanismTest: {
    mechanismUnderTest: 'That this is the inline-axis face of the frozen-derived-geometry defect: capture reads resolved computed values, so the recreation replays pixels that were correct only at the width they were sampled at.',
    whyThisIsTestedWithinPageNotBetweenPages: 'A between-page predictor is hostage to class balance and ties the baseline on an all-divergent sample no matter how correct it is. The mechanism, unlike the label, forbids a specific cell: error must collapse at a width the capture actually sampled. That is a within-page contrast and needs no clean pages at all, so it survives the very class imbalance that made requirement 1 null.',
    sampledWidths: [1920, 1440, 768, 390, 320],
    sampledWidthsSource: 'src/cli.rs:77 default --viewports 1920x1080,1440x900,768x1024,390x844,320x568 (read only, not edited)',
    probeWidths: [400, 450, 480, 600, 661, 800, 1076, 1440],
    onlySampledWidthInSweep: 1440,
    tolerancePx: 2,
    level1_naive: {
      test: 'Is width-mismatch % at its minimum at the sampled width 1440?',
      result: '8 of 8 testable pages',
      REJECTED_because: 'Confounded. 1440 is not only the sole sampled width in the sweep, it is also the widest probe. Any error that merely shrinks as viewports widen puts its minimum there automatically. This number must not be quoted as support.',
    },
    level2_rawDiscontinuity: {
      test: 'Does the LARGEST step in the sweep fall on the 1076->1440 boundary, i.e. is there a step change at the sampled width rather than a smooth decline?',
      poweredPages: powered.map(([s]) => s),
      underpowered: Object.entries(disc).filter(([, v]) => v.testable && !v.poweredForStepTest).map(([s, v]) => `${s} (already at ${v.pctAt1076}% by 1076, no headroom left to fall)`),
      powerThresholdNote: 'The >5% at 1076 cut was chosen AFTER seeing the data and is declared as post hoc. The observed plateau floors were 0, 2.3 and 2.8, so 5.0 sits clear of all three.',
      result: `${rawOK.length} of ${powered.length} powered pages [${rawOK.join(', ')}]`,
      cernCaveat: 'cern passes only on an exact tie with its own 400->450 step, not strictly.',
      counterexample: 'vuejs: 6.8% at 1076 -> 6.7% at 1440, a step of 0.1, with its largest step at 661->800 instead.',
      exactBinomialPUnderNull: '1.84e-3 against a null that the largest step lands on any of the 7 boundaries',
      STILL_REJECTED_because: 'Also confounded. 1076->1440 is the widest step in the sweep at 364px against 30-276px elsewhere, so a change proportional to viewport travel lands its largest raw step there for reasons unrelated to sampling.',
    },
    level3_spanNormalised: {
      test: 'Same, but normalised to % per pixel of viewport travel, which removes the widest-step confound.',
      perPage: Object.fromEntries(powered.map(([s, v]) => [s, {
        atSampledBoundary: v.normStepAtSampled,
        bestElsewhere: v.maxNormStepElsewhere,
        signatureHolds: v.largestNormStepIsAtSampledBoundary,
      }])),
      result: `${normOK.length} of ${powered.length} powered pages [${normOK.join(', ')}]`,
      exactBinomialPUnderNull: '5.37e-1, i.e. indistinguishable from chance',
      VERDICT: 'THE MECHANISM CLAIM IS NOT SUPPORTED AT POPULATION LEVEL. Once both confounds are removed the sampled-width signature survives on exactly one of five powered pages. I registered this mechanism myself and it does not hold.',
    },
    theOnePageThatDoesShowIt: {
      site: 'sveltedev',
      evidence: 'Width mismatch is 85.0% at 1076 and 5.0% at 1440 - a collapse of 80 points at exactly the sampled width, while its largest step anywhere else in the sweep is 5.9 points. Span-normalised it is 0.220 %/px at the sampled boundary against 0.090 %/px at its best rival step, a factor of 2.4.',
      reading: 'On this page the frozen-derived-geometry account is not merely consistent with the data, it is the only account that explains an 80 point discontinuity landing on the one width in the sweep the capture actually sampled. The mechanism is real; it is just not general.',
    },
    theStrongestEvidenceAgainstOneSharedRootCause: {
      site: 'cern (info.cern.ch/hypertext/WWW/TheProject.html)',
      evidence: 'The page has no stylesheet at all, so there is no authored fluid length anywhere for the capture to freeze, yet it still diverges: 12 source-only overlap pairs and width mismatch of 7.4-14.8% across the sweep.',
      reading: 'A defect that appears on a page with nothing to freeze cannot be caused by freezing. Whatever produces cern is a second, independent fault. One generator fix should therefore NOT be expected to close both.',
    },
    consequenceForTheDeveloper: 'Do not treat the width divergence as a downstream symptom of the resize defect. The evidence supports frozen geometry on 1 of 5 powered pages and actively contradicts it on cern. Fixing frozen derived geometry should be predicted to fix sveltedev and to leave the other pages roughly where they are; if that prediction fails in either direction it is new information.',
  },
  requirement4_whatTheRuleBuys: {
    proposedUse: 'A cheap per-element width check that flags a page before the expensive overlap pass.',
    verdict: 'NOT EARNED. The rule has never once been observed to say CLEAN on an informative page. Its false-positive rate is not merely high, it is unmeasured, because no informative page in either batch was clean. A gate whose negative arm has zero observations cannot be trusted to skip work.',
    whatIsEarnedInstead: 'Report width mismatch as a diagnostic number beside the overlap counts. It is cheap, it is strictly more informative than the overlap count alone (cern shows 0 recreation-only pairs but 14.8% width error), and it costs nothing in false negatives because nothing is skipped on its say-so.',
    measuredCost: '84-308 ms per side for 8 widths, resize-dominated (195 ms resize vs 113 ms collect on the largest page). Both sides 168-616 ms, i.e. 3.8-14.0% of the 4400 ms COMPARISON_DEADLINE_MS in backtest/src/deadline.rs:5. Cost scales with probe-width count, not page size; 3 widths is about 5%.',
  },
  requirement5_exposure: {
    note: 'Width mismatch is reported over shared texts present on BOTH sides, so it is a rate rather than a raw count and is already exposure-normalised. Denominators are given per width.',
    sharedTextsPerPageAtNarrowestAndWidest: Object.fromEntries(Object.entries(sig)
      .filter(([, v]) => v.perWidth && v.perWidth[400])
      .map(([s, v]) => [s, { at400: v.perWidth[400].sharedTexts, at1440: v.perWidth[1440].sharedTexts }])),
    directionSplit: 'Recorded per width in wf/wf-signature.json as recreationWider vs sourceWider, so an over-wide recreation is never conflated with an under-wide one.',
  },
  instrumentGaps: [
    'The probe sweep has exactly one sampled width in it, and that width is also its widest point. This was a design error on my part: it made the naive signature test unfalsifiable. A sweep that straddled 768 on both sides would test the mechanism far better, since 768 is a sampled width in the interior rather than at an endpoint.',
    'Three of eight testable pages plateaued near the 2px tolerance floor before reaching 1440 and could not contribute to the step test at all.',
    'danluu is flat at 0% across every width, so it has no minimum to locate and is untestable for the signature even though it is a real clean measurement.',
    'gnu returned zero probe candidates on both sides, a source-side block against automation, and yields nothing for either test.',
    'The instrument compares resolved widths only. It cannot tell an authored length from an engine-derived one, which is the single measurement that would settle the shared-root-cause question outright; that needs walking the source page cssRules.',
  ],
  outOfScope: 'No file under src/, backtest/ or browser/ was read for anything but the sampled-width constant, and none was modified. No capture was run this turn. The predicate, the 2px tolerance and the width sweep are unchanged from the locked run.',
};

fs.writeFileSync(F, JSON.stringify(d, null, 2));
const after = { b1: sha(d.preRegistration), b2: sha(d.preRegistrationBatch2) };
console.log('prereg block hashes  before -> after');
console.log('  batch1 ' + before.b1 + ' -> ' + after.b1 + '  ' + (before.b1 === after.b1 ? 'UNCHANGED' : 'MUTATED'));
console.log('  batch2 ' + before.b2 + ' -> ' + after.b2 + '  ' + (before.b2 === after.b2 ? 'UNCHANGED' : 'MUTATED'));
console.log('\nbatch1 ' + b1.verdict);
console.log('\nbatch2 ' + b2.verdict);
console.log('\nmechanism level3: ' + normOK.length + '/' + powered.length + ' -> NOT SUPPORTED');
