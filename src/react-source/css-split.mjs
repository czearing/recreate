import postcss from 'postcss';
import { formatCss } from './css-format.mjs';

const classNames = (selector) => [
  ...String(selector || '').matchAll(/\.(-?[_a-zA-Z]+[_a-zA-Z0-9-]*)/g),
].map((match) => match[1]);

function wrappedRule(rule, layer) {
  const clone = rule.clone();
  if (clone.selector) clone.selector = clone.selector.replace(/\bsvg\b/g, 'img');
  let source = clone.toString();
  let parent = rule.parent;
  while (parent?.type === 'atrule') {
    source = `@${parent.name}${parent.params ? ` ${parent.params}` : ''}{${source}}`;
    parent = parent.parent;
  }
  return layer ? `@layer ${layer}{${source}}` : source;
}

function globalRule(rule) {
  const selector = String(rule.selector || '').trim();
  return (
    !classNames(selector).length &&
    (
      /^(?:\*|html|body|:root)(?:$|\b|[,:*\s>+~[])/.test(selector) ||
      selector.startsWith(':') ||
      selector.includes('[data-') ||
      selector.includes('[aria-')
    )
  );
}

export function splitCssByComponent(cssSources, definitions) {
  const ownersByClass = new Map();
  for (const definition of definitions) {
    for (const className of definition.classes || []) {
      const owners = ownersByClass.get(className) || new Set();
      owners.add(definition.name);
      ownersByClass.set(className, owners);
    }
  }
  const shared = new Set();
  const byComponent = new Map();
  const layers = [];
  const candidates = [];
  let totalRuleCount = 0;
  let keptRuleCount = 0;
  for (const source of cssSources) {
    const root = postcss.parse(source);
    root.walkAtRules((atRule) => {
      if (!['font-face', 'keyframes', '-webkit-keyframes'].includes(atRule.name)) return;
      shared.add(wrappedRule(atRule));
    });
    root.walkRules((rule) => {
      totalRuleCount += 1;
      const classes = classNames(rule.selector);
      const owners = new Set(classes.flatMap((className) =>
        [...(ownersByClass.get(className) || [])]));
      if (!owners.size && !globalRule(rule)) return;
      candidates.push({
        canonical: wrappedRule(rule),
        order: totalRuleCount,
        owners,
      });
    });
  }
  const lastByRule = new Map(candidates.map((candidate) =>
    [candidate.canonical, candidate]));
  for (const candidate of candidates) {
    if (lastByRule.get(candidate.canonical) !== candidate) continue;
    keptRuleCount += 1;
    const layer = `siteSpec${String(candidate.order).padStart(4, '0')}`;
    layers.push(layer);
    const sourceText = `@layer ${layer}{${candidate.canonical}}`;
    if (candidate.owners.size !== 1) {
      shared.add(sourceText);
      continue;
    }
    const [owner] = candidate.owners;
    const rules = byComponent.get(owner) || new Set();
    rules.add(sourceText);
    byComponent.set(owner, rules);
  }
  return {
    sharedCss: formatCss(`@layer ${layers.join(',')};\n${[...shared].join('\n')}`),
    componentCss: new Map([...byComponent].map(([name, rules]) =>
      [name, formatCss([...rules].join('\n'))])),
    totalRuleCount,
    keptRuleCount,
  };
}
