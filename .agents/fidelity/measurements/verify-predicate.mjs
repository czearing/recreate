// Independent check that the overlap predicate this run uses is byte-identical
// to the one overlap-multi-site.json was produced with.
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';

function extract(file) {
  const t = readFileSync(file, 'utf8');
  const i = t.indexOf('const PROBE = `');
  const j = t.indexOf('})()`;', i);
  const d = t.slice(i, j + 6);
  return d.slice(d.indexOf('`') + 1, d.lastIndexOf('`'));
}

const prior = extract('overlap-multi-site.mjs');
const sha = createHash('sha256').update(prior).digest('hex').slice(0, 16);
console.log('prior PROBE len', prior.length, 'sha', sha);
const mod = await import('./overlap-property-census.mjs');
console.log('this run PROBE sha', mod.PROBE_SHA);
console.log('IDENTICAL:', mod.PROBE_SHA === sha);
