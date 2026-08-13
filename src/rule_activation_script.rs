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
  // Gate versus carrier, which is the split that matters, and it does not follow
  // conditionality. The line is *who answers the condition*: the user agent, or the
  // document. A recreation reproduces the document and not the agent, so only an
  // agent-answered condition may be evaluated now and dropped. `@supports` asks the engine
  // about its own feature support, which is fixed for the run and is not a property of the
  // page — re-emitting it would make the recreation re-ask the viewing engine a question
  // the capturing engine already answered, and drop declarations the captured page had.
  // Every other condition is answered by the document and is re-answered by whoever views
  // the recreation: a media condition by the viewport, and a container condition by the
  // used inline-size of the nearest ancestor with `container-type`, which layout produces
  // afresh on every resize and which two instances of one component answer differently in
  // the same paint. Baking such a condition away publishes the branch that happened to hold
  // as though the author had written it unconditionally. `@layer` and `@scope` carry cascade
  // position and proximity that outrank specificity and that no flattened copy can express.
  // So carriers are re-emitted verbatim around every member.
  const carrier = prelude => !prelude.startsWith('@supports');
  // The platform's own line between a rule that groups style rules and one that merely has
  // children. @keyframes exposes cssRules, but its children are keyframe selectors rather
  // than rules the cascade ever resolves. Asking "does it have children" instead gets both
  // directions wrong: descending records percentages as authored rules, and skipping
  // records nothing, dropping the block every animation-name refers to. One owner, because
  // the walk and the recorder each need this answer and a second copy of it drifts.
  const grouping = rule => rule instanceof CSSGroupingRule;
  // A grouping rule is never recorded in its own right. Its cssText already serialises
  // every rule nested inside it, and once each member carries its own carrier chain the
  // group's text holds nothing its members do not. That also removes the only records a
  // probe could never reach: a grouping rule has no selector, so its `active` would be the
  // default it was handed rather than anything observed, and a @media nested in a feature
  // query that was false at capture would ship as live.
  const flattenRules = (rules, media = null, gates = [], carriers = []) => {
    const entries = [];
    for (const rule of Array.from(rules || [])) {
      if (!grouping(rule)) {
        entries.push({ rule, media, gates, carriers, active: true });
        continue;
      }
      const prelude = preludeOf(rule);
      const held = carrier(prelude);
      entries.push(...flattenRules(
        rule.cssRules,
        rule.type === CSSRule.MEDIA_RULE
          ? (media ? `(${media}) and (${rule.conditionText})` : rule.conditionText)
          : media,
        held || !prelude ? gates : gates.concat(prelude),
        held && prelude ? carriers.concat(prelude) : carriers
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
      if (!entry.gates.length || !entry.rule.selectorText) continue;
      const selector = staticSelector(entry.rule.selectorText);
      if (!selector) continue;
      entry.selector = selector;
      entry.probe = `--recreate-probe-${probes.length}`;
      probes.push(entry);
    }
    if (!probes.length) return entries;
    const style = document.createElement('style');
    style.textContent = probes.map(entry => entry.gates.reduceRight(
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
