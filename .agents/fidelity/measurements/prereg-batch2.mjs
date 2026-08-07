// SECOND pre-registration. Written and committed BEFORE batch-2 capture.
// Batch 1 scored the rule at 5/5 but the always-divergent baseline also
// scored 5/5, because every informative batch-1 page was divergent. A tie
// with the baseline is no evidence. Batch 2 exists solely to supply
// informative CLEAN pages, without which the rule cannot be discriminated.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const batch2 = {
  registeredAtUtc: new Date().toISOString(),
  registeredAtHeadSha: process.argv[2] || 'UNKNOWN',
  status: 'LOCKED BEFORE BATCH-2 CAPTURE. None of these pages had been captured, built, served or measured when this block was written and committed.',

  whyASecondBatchExists:
    'Batch 1 produced 5 informative pages and ALL 5 were divergent, so the width-fidelity rule (5/5) tied the always-divergent baseline (5/5) and the sample carries no discriminating information. The one predicted-clean page that stayed clean, danluu, was graded weak by the inherited rule because the overlap predicate never fired on it at any width. Batch 2 therefore targets a harder and more specific case: pages that DO have dense inline structure, so the predicate can fire, but whose widths are NOT fluid, so the mechanism predicts no divergence.',

  whatWouldFalsifyTheRule:
    'A page whose width-mismatch is exactly 0 at all eight widths but which nevertheless shows a nonzero recreation-only or source-only pair count at any width, or a page with nonzero width-mismatch that is nonetheless clean in both directions at every width. Either outcome breaks the zero-versus-nonzero boundary.',

  decisionRuleUnchanged:
    'Identical to batch 1 and to overlap-property-census.json: width-mismatch percent exactly 0 at all eight probe widths means CLEAN, any nonzero width means DIVERGENT, tolerance 2px, outcome counts both directions. Nothing is re-tuned. The batch-1 results are reported whatever batch 2 shows.',

  predictedLabels: {
    lobsters: {
      url: 'https://lobste.rs/',
      stack: 'Ruby on Rails, hand-authored CSS, table-driven story list',
      predicted: 'CLEAN',
      reasoning: 'Dense inline rows (vote arrows, byline, tag links) so the predicate will fire, but the story list is a native table whose column widths the engine derives from content rather than from an authored fluid length. Under the mechanism there is no authored percentage to freeze.',
    },
    gnu: {
      url: 'https://www.gnu.org/',
      stack: 'static HTML, long-lived hand-authored CSS',
      predicted: 'CLEAN',
      reasoning: 'Old-school simple stylesheet with inline nav and link lists. Chosen as a second predicted-negative with real inline structure.',
    },
    w3c: {
      url: 'https://www.w3.org/',
      stack: 'static site, hand-authored CSS',
      predicted: 'DIVERGENT',
      reasoning: 'Included deliberately as a predicted POSITIVE inside batch 2 so the batch is not all one class. The redesigned w3.org uses a fluid grid with percentage-based cards.',
    },
    sourcehut: {
      url: 'https://sourcehut.org/',
      stack: 'Go-served static pages, Bootstrap-style grid',
      predicted: 'DIVERGENT',
      reasoning: 'Second predicted positive. A Bootstrap-style row/col grid resolves columns to percentages, which is exactly the freezable construct the mechanism names.',
    },
  },

  classBalance: '2 predicted CLEAN, 2 predicted DIVERGENT, of 4 registered pages.',
  deviationsPolicy: 'Unchanged. A page the tool cannot process is reported with its exact error and removed from the denominator, never relabelled. No page added after capture begins.',
};

const prev = JSON.parse(readFileSync('width-fidelity-prereg.json', 'utf8'));
prev.preRegistrationBatch2 = batch2;
writeFileSync('width-fidelity-prereg.json', JSON.stringify(prev, null, 2));
const h = createHash('sha256').update(JSON.stringify(batch2.predictedLabels)).digest('hex').slice(0, 16);
writeFileSync('prereg-lock-batch2.txt', `batch2 predictedLabels sha256/16 = ${h}\nregistered ${batch2.registeredAtUtc}\n`);
console.log('batch2 lock', h);
console.log(Object.entries(batch2.predictedLabels).map(([k, v]) => `${k}=${v.predicted}`).join(' '));
