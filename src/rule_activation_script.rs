/// Whether an authored rule actually applies is a browser decision, not a parse decision.
///
/// `@supports` and `@container` expose their nested rules through the CSSOM even when
/// their condition does not match, so a walk that reads rule text alone records dead
/// declarations as authored ones. No API answers a container query, and evaluating each
/// at-rule family separately would need a new branch for every conditional at-rule the
/// platform adds. Instead every nested rule is re-emitted under its own at-rule prelude
/// chain with a sentinel custom property, and the browser decides: a rule is active when
/// at least one element it selects receives the sentinel. One code path covers `@media`,
/// `@supports`, `@container` and anything conditional that follows them.
pub const SOURCE: &str = r#"
  const dynamicStatePattern = /:(hover|focus-visible|focus-within|focus|active)\b/g;
  // A probe rule must be able to match right now, so states the page is not currently in
  // are stripped. What is left measures the at-rule condition and nothing else.
  const staticSelector = selector => selector
    .replace(dynamicStatePattern, '')
    .replace(/::[\w-]+/g, '')
    .split(',')
    .map(part => part.trim())
    .filter(Boolean)
    .join(', ');
  const preludeOf = rule => {
    const brace = rule.cssText.indexOf('{');
    return brace < 0 ? '' : rule.cssText.slice(0, brace).trim();
  };
  // Conditionality answers whether a rule is active. It does not answer what wrapper the
  // rule needs to be re-emitted inside, and one predicate cannot serve both: @layer and
  // @scope group unconditionally, so no probe is needed, but their prelude is cascade
  // position that no flattened copy can carry. So conditions drive activation and
  // preludes drive emission, and a group contributes to exactly one of them.
  const conditional = rule =>
    typeof rule.conditionText === 'string' && !!rule.conditionText.trim();
  const flattenRules = (rules, media = null, conditions = [], preludes = []) => {
    const entries = [];
    for (const rule of Array.from(rules || [])) {
      entries.push({ rule, media, conditions, preludes, active: true });
      // Only a CSSGroupingRule holds style rules that stand on their own. @keyframes also
      // exposes cssRules, but its children are keyframe selectors rather than rules the
      // cascade ever resolves, so descending into one records percentages as authored
      // rules and re-emits each keyframe as a stylesheet of its own.
      if (!(rule instanceof CSSGroupingRule)) continue;
      const prelude = preludeOf(rule);
      const gates = conditional(rule);
      const nestedMedia = rule.type === CSSRule.MEDIA_RULE
        ? (media ? `(${media}) and (${rule.conditionText})` : rule.conditionText)
        : media;
      entries.push(...flattenRules(
        rule.cssRules,
        nestedMedia,
        gates && prelude ? conditions.concat(prelude) : conditions,
        !gates && prelude ? preludes.concat(prelude) : preludes
      ));
    }
    return entries;
  };
  // A container condition resolves per element, so the answer can differ between elements
  // one selector matches. Reading every match of every rule is unbounded on a large page,
  // so the search stops once an answer is found and after a fixed number of candidates.
  const probeMatches = entry => {
    let elements;
    try { elements = document.querySelectorAll(entry.selector); } catch { return true; }
    let checked = 0;
    for (const element of elements) {
      if (getComputedStyle(element).getPropertyValue(entry.probe).trim()) return true;
      if (++checked >= 32) break;
    }
    return false;
  };
  const activateEntries = entries => {
    const probes = [];
    for (const entry of entries) {
      if (!entry.conditions.length || !entry.rule.selectorText) continue;
      const selector = staticSelector(entry.rule.selectorText);
      if (!selector) continue;
      entry.selector = selector;
      entry.probe = `--recreate-probe-${probes.length}`;
      probes.push(entry);
    }
    if (!probes.length) return entries;
    const style = document.createElement('style');
    style.textContent = probes.map(entry => entry.conditions.reduceRight(
      (inner, condition) => `${condition}{${inner}}`,
      `${entry.selector}{${entry.probe}:1}`
    )).join('\n');
    // Injected once with no interleaved reads, so every probe below shares one style
    // recalculation rather than forcing one per rule.
    (document.head || document.documentElement).appendChild(style);
    for (const entry of probes) entry.active = probeMatches(entry);
    style.remove();
    return entries;
  };
"#;

#[cfg(test)]
#[path = "rule_activation_tests.rs"]
mod tests;
