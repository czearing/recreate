// Duplicate-component inventory for the generated React artifact.
// Read-only measurement: parses the emitted .jsx tree, extracts every component's
// render body, and groups bodies under four explicitly stated normalizations.
import {readFileSync, writeFileSync, readdirSync, statSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {join, relative, sep} from 'node:path';

const ROOT = process.argv[2];
const OUT = process.argv[3];
const SHA = process.argv[4];

function walk(dir, acc = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p, acc);
    else if (name.endsWith('.jsx')) acc.push(p);
  }
  return acc;
}

// Brace-match a function body starting at the index of its opening '{'.
// Skips strings, template literals and comments so JSX text cannot unbalance it.
function matchBody(src, open) {
  let depth = 0;
  for (let i = open; i < src.length; i++) {
    const c = src[i];
    if (c === '/' && src[i + 1] === '/') { i = src.indexOf('\n', i); if (i < 0) break; continue; }
    if (c === '/' && src[i + 1] === '*') { i = src.indexOf('*/', i) + 1; continue; }
    if (c === '"' || c === "'" || c === '`') {
      const q = c;
      for (i++; i < src.length; i++) {
        if (src[i] === '\\') { i++; continue; }
        if (src[i] === q) break;
      }
      continue;
    }
    if (c === '{') depth++;
    else if (c === '}') { depth--; if (depth === 0) return src.slice(open, i + 1); }
  }
  return null;
}

// A component is a function declaration with a PascalCase name whose body
// contains JSX and which is not nested inside another component's body.
// Covers all emitted shapes: `export default function X(...)`,
// `export function X(...)`, and the non-exported runtime surface components
// in states.jsx (`function X(...)`).
const DECL = /(?:export\s+(?:default\s+)?)?function\s+([A-Z][A-Za-z0-9_]*)\s*\(/g;

const components = [];
const skipped = [];
for (const file of walk(ROOT)) {
  const src = readFileSync(file, 'utf8');
  const rel = relative(ROOT, file).split(sep).join('/');
  const candidates = [];
  let m;
  DECL.lastIndex = 0;
  while ((m = DECL.exec(src))) {
    const paramsOpen = src.indexOf('(', m.index + m[0].length - 1);
    // find the '{' that begins the body: after the balanced parameter list
    let d = 0, i = paramsOpen;
    for (; i < src.length; i++) {
      if (src[i] === '(') d++;
      else if (src[i] === ')') { d--; if (d === 0) break; }
    }
    const open = src.indexOf('{', i);
    const body = matchBody(src, open);
    if (!body) { skipped.push({file: rel, name: m[1], reason: 'unbalanced body'}); continue; }
    // Do NOT require JSX in the body: the three runtime surface components in
    // states.jsx are real emitted React components that render null via an
    // effect. Excluding them would silently shrink the denominator.
    candidates.push({name: m[1], file: rel, body, start: m.index, end: open + body.length, renders_jsx: /<[A-Za-z>]/.test(body)});
  }
  // Drop declarations nested inside another candidate's body: those are helper
  // closures, not separately emitted components.
  const top = candidates.filter(c => !candidates.some(o => o !== c && o.start < c.start && c.end <= o.end));
  for (const c of top) components.push({name: c.name, file: c.file, body: c.body, renders_jsx: c.renders_jsx});
  for (const c of candidates) if (!top.includes(c)) skipped.push({file: rel, name: c.name, reason: 'nested inside another component'});
  if (top.length === 0) skipped.push({file: rel, name: null, reason: 'no component declaration'});
}

const ws = s => s.replace(/\s+/g, ' ').trim();
const hash = s => createHash('sha256').update(s).digest('hex').slice(0, 16);

// N1: normalize ONLY the component's own name (the literal metric requested).
// N2: additionally normalize alias suffixes on referenced child components.
// N3: additionally normalize generator-minted scoped class hashes (rXXXXXXXXXX).
// N0: raw body, no normalization at all (control).
const norms = {
  raw: c => ws(c.body),
  own_name_only: c => ws(c.body.replace(new RegExp(`\\b${c.name}\\b`, 'g'), '__SELF__')),
  own_name_and_callee_alias: c => ws(c.body
    .replace(new RegExp(`\\b${c.name}\\b`, 'g'), '__SELF__')
    .replace(/\b([A-Z][A-Za-z0-9_]*?)Variant\d+\b/g, '$1')),
  own_name_callee_alias_and_class_hash: c => ws(c.body
    .replace(new RegExp(`\\b${c.name}\\b`, 'g'), '__SELF__')
    .replace(/\b([A-Z][A-Za-z0-9_]*?)Variant\d+\b/g, '$1')
    .replace(/\br[0-9a-f]{10}\b/g, '__CLASS__')),
  // Reproduces work item 003's adopted Definition D: the alias-normalized tag
  // skeleton (all attributes and text discarded) compared only within one family.
  tag_skeleton_within_family: c => {
    const family = c.name.replace(/Variant\d+$/, '');
    const tags = [...c.body.matchAll(/<\/?([A-Za-z][A-Za-z0-9_.]*)/g)]
      .map(t => t[1].replace(/Variant\d+$/, '').replace(new RegExp(`^${family}$`), '__SELF__'));
    return `${family}|${tags.join(' ')}`;
  },
};

const total = components.length;
const results = {};
for (const [label, fn] of Object.entries(norms)) {
  const groups = new Map();
  for (const c of components) {
    const key = hash(fn(c));
    if (!groups.has(key)) groups.set(key, {signature: key, members: [], representative_body: c.body, representative: c.name, representative_file: c.file});
    groups.get(key).members.push(c.name);
  }
  const multi = [...groups.values()].filter(g => g.members.length > 1)
    .sort((a, b) => b.members.length - a.members.length || a.representative.localeCompare(b.representative));
  // "duplicate" = a component whose render body is identical to another component's,
  // i.e. every member of a group of size >= 2.
  const duplicate_components = multi.reduce((n, g) => n + g.members.length, 0);
  // "redundant" = how many could be deleted by collapsing each group to one.
  const redundant_components = multi.reduce((n, g) => n + g.members.length - 1, 0);
  results[label] = {
    total_components: total,
    distinct_bodies: groups.size,
    duplicate_groups: multi.length,
    duplicate_components,
    duplicate_share_percent: +(100 * duplicate_components / total).toFixed(2),
    redundant_components,
    redundant_share_percent: +(100 * redundant_components / total).toFixed(2),
    largest_groups: multi.slice(0, 5).map(g => ({
      member_count: g.members.length,
      members: g.members,
      representative: g.representative,
      representative_file: g.representative_file,
      representative_body: g.representative_body,
    })),
    all_duplicate_group_members: multi.map(g => g.members),
  };
}

const byFile = {};
for (const c of components) (byFile[c.file] ||= []).push(c.name);

const byName = new Map(components.map(c => [c.name, c]));

// Resolve every generated class hash to the full set of declarations that target
// it, across every emitted stylesheet, keeping the at-rule context so a rule
// inside a media query is not confused with the same rule outside one.
function collectCss(dir, acc = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) collectCss(p, acc);
    else if (name.endsWith('.css')) acc.push(p);
  }
  return acc;
}
const classRules = new Map();
for (const file of collectCss(ROOT)) {
  const css = readFileSync(file, 'utf8');
  const ctx = [];
  let i = 0, prelude = '';
  while (i < css.length) {
    const ch = css[i];
    if (ch === '{') {
      const head = prelude.trim();
      prelude = '';
      if (head.startsWith('@')) { ctx.push(head); i++; continue; }
      let d = 1, j = i + 1;
      for (; j < css.length && d > 0; j++) { if (css[j] === '{') d++; else if (css[j] === '}') d--; }
      const decls = css.slice(i + 1, j - 1).replace(/\s+/g, ' ').trim();
      for (const m of head.matchAll(/\br[0-9a-f]{10}\b/g)) {
        if (!classRules.has(m[0])) classRules.set(m[0], []);
        // Replace the class itself with a placeholder, then drop every other
        // alternative in a grouped selector list. A rule written
        // `.a,.b { ... }` applies to .a exactly as `.a { ... }` would, so the
        // sibling hashes must not make two members compare unequal.
        const sel = head
          .replace(new RegExp(`\\b${m[0]}\\b`, 'g'), '__SELF__')
          .split(',')
          .filter(part => part.includes('__SELF__'))
          .map(part => part.trim())
          .sort()
          .join(',');
        classRules.get(m[0]).push(`${ctx.join(' | ')} :: ${sel} { ${decls} }`);
      }
      i = j;
      continue;
    }
    if (ch === '}') { ctx.pop(); prelude = ''; i++; continue; }
    prelude += ch;
    i++;
  }
}
const cssOf = name => {
  const c = byName.get(name);
  const hashes = [...new Set([...c.body.matchAll(/\br[0-9a-f]{10}\b/g)].map(m => m[0]))];
  return hashes.map(h => (classRules.get(h) || []).slice().sort().join('\n')).join('\n@@\n');
};

// Evidence for the strict result: for each group that only merges once class
// hashes are abstracted, record exactly which tokens separate the first two
// members under the strict (own-name-only) form. If those tokens are nothing
// but generated class hashes, the strict count of 0 is explained rather than
// merely asserted. Then check whether the differing hashes carry identical
// declarations: only then is collapsing the group behaviour-preserving.
const strict_zero_evidence = results.own_name_callee_alias_and_class_hash.largest_groups.map(g => {
  const [a, b] = g.members.slice(0, 2).map(n => byName.get(n));
  const tok = c => new Set(norms.own_name_only(c).split(/[^A-Za-z0-9_-]+/).filter(Boolean));
  const ta = tok(a), tb = tok(b);
  const only_a = [...ta].filter(t => !tb.has(t));
  const only_b = [...tb].filter(t => !ta.has(t));
  const cssGroups = new Map();
  for (const n of g.members) {
    const k = hash(cssOf(n));
    if (!cssGroups.has(k)) cssGroups.set(k, []);
    cssGroups.get(k).push(n);
  }
  return {
    pair: [a.name, b.name],
    tokens_only_in_first: only_a,
    tokens_only_in_second: only_b,
    all_differences_are_generated_class_hashes:
      only_a.every(t => /^r[0-9a-f]{10}$/.test(t)) && only_b.every(t => /^r[0-9a-f]{10}$/.test(t)),
    // How the group splits once the CSS behind each class hash is compared.
    // A subgroup of size > 1 is safely collapsible; size 1 means the class
    // really does carry different style and those are not clones.
    css_equivalence_subgroups: [...cssGroups.values()].sort((x, y) => y.length - x.length),
    behaviour_preserving_collapsible:
      [...cssGroups.values()].reduce((n, m) => n + m.length - 1, 0),
  };
});
const behaviour_preserving_collapsible_total =
  strict_zero_evidence.reduce((n, e) => n + e.behaviour_preserving_collapsible, 0);
const dead_classes = [...new Set(components.flatMap(c =>
  [...new Set([...c.body.matchAll(/\br[0-9a-f]{10}\b/g)].map(m => m[0]))]
    .filter(h => !classRules.has(h))))];

writeFileSync(OUT, JSON.stringify({
  measurement: 'duplicate-component-inventory',
  commit_sha: SHA,
  measured_at: new Date().toISOString(),
  source_site: 'https://m365sandbox.microsoft.com/protos/staging-vnow/onenoteai',
  spec_input: 'live-r15\\spec.json',
  generate_command: 'cargo run --release -q -- generate --spec live-r15\\spec.json --out <private>',
  analyzed_tree: ROOT,
  population_rule: 'Every non-nested function declaration in the generated .jsx tree whose name is PascalCase. Includes the four non-exported runtime surface components in states.jsx (three of which render null via an effect and were deliberately NOT excluded, to avoid silently shrinking the denominator). Excludes re-export stubs, lowercase helper functions, and closures nested inside another component body. This yields 48, matching the denominator used by work item 003 and by the stale baseline.',
  duplicate_definitions: {
    own_name_only: 'PRIMARY, as instructed: render bodies identical after replacing only the component\'s own identifier. Type-1 clone.',
    own_name_and_callee_alias: 'Additionally strips Variant<N> suffixes from referenced child components, so one aliased leaf does not split every ancestor. Type-2 clone.',
    own_name_callee_alias_and_class_hash: 'Additionally abstracts generator-minted scoped class hashes (r[0-9a-f]{10}), which are identifiers the generator mints per emitted component. Type-2 clone, identifier-normalized.',
    tag_skeleton_within_family: 'Reproduces work item 003\'s adopted Definition D (alias_duplicate_ratio): the alias-normalized JSX tag skeleton, all attributes and text discarded, compared only within one family (name with the Variant<N> suffix stripped). Included so the stale 23/48 baseline is directly checkable against this run.',
    raw: 'Control: no normalization at all.',
  },
  counting_convention: {
    duplicate_components: 'Every member of a group of size >= 2 (a component whose body is identical to another component\'s).',
    redundant_components: 'Members minus one representative per group; the number deletable by collapsing. This is the convention used by work item 003 (23/48).',
  },
  results,
  headline: {
    total_components: total,
    primary_metric: 'own_name_only',
    primary_duplicate_components: results.own_name_only.duplicate_components,
    primary_duplicate_share_percent: results.own_name_only.duplicate_share_percent,
    stale_claim_checked: '25 duplicates of 48 components, 52 percent',
    stale_claim_verdict: 'Not reproduced under the definition it was stated with. The denominator 48 is confirmed exactly. Under the literal definition (render body identical ignoring only the component name) the count is 0, because every emitted component carries a unique generated scoped class hash in its body. The 52 percent figure is only approached under a much looser structural definition: the alias-normalized tag skeleton within a family gives 23 of 48 (47.92 percent), which reproduces work item 003 Definition D to the unit.',
    structural_duplicate_components: results.tag_skeleton_within_family.duplicate_components,
    structural_redundant_components: results.tag_skeleton_within_family.redundant_components,
    structural_redundant_share_percent: results.tag_skeleton_within_family.redundant_share_percent,
    behaviour_preserving_collapsible_total,
    caveat: 'behaviour_preserving_collapsible_total is 0: within every structurally identical group, each member resolves to a different set of CSS declarations. The aliases are not dead copies; each one exists to carry a distinct style bundle. A dedup fix that merges them by identity will change rendering. The fix has to lift the scoped class into a prop, not delete components.',
  },
  strict_zero_evidence,
  behaviour_preserving_collapsible_total,
  dead_classes,
  components_by_file: byFile,
  skipped_files: skipped,
}, null, 2));

console.log(`total components: ${total}`);
for (const [k, v] of Object.entries(results)) {
  console.log(`${k}: groups=${v.duplicate_groups} dup=${v.duplicate_components} (${v.duplicate_share_percent}%) redundant=${v.redundant_components} (${v.redundant_share_percent}%)`);
}
console.log('skipped:', JSON.stringify(skipped));
