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
  // A probe rule must be able to match right now, so the states the page is not currently in
  // are reduced away and the boxes it would generate are dropped. What is left measures the
  // at-rule condition and nothing else. Both reductions are the reader's, because a probe
  // built by cutting selector text apart measures whatever the fragment happens to match.
  const staticSelector = selector => selectorMembers(selector)
    .map(member => restingSelector(withoutGeneratedBoxes(member)))
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
  const flattenRules = (rules, media = null, gates = [], carriers = [], seen = new Set()) => {
    const entries = [];
    for (const rule of Array.from(rules || [])) {
      // The one rule whose child is a whole sheet rather than a rule list. It is not a
      // grouping rule, so the test above cannot reach it, and the sheet it names appears in
      // no collection the caller enumerates: CSSOM builds those from owner *nodes*, and an
      // imported sheet has an owner *rule*. It is consumed rather than recorded, because
      // re-emitting `@import` would refetch the sheet and apply every rule in it twice.
      if (rule instanceof CSSImportRule) {
        entries.push(...enterSheet(rule.styleSheet, media, gates, carriers, seen));
        continue;
      }
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
        held && prelude ? carriers.concat(prelude) : carriers,
        seen
      ));
    }
    return entries;
  };
  // A condition can also live on the sheet rather than in it — the `media` attribute of a
  // `<style>` or `<link>`, or the query trailing an `@import` prelude. It conditions every
  // rule the sheet holds exactly as an enclosing `@media` block would, but it is not a rule,
  // so it never appears in `cssRules`. A walk seeded with nothing therefore records those
  // rules as unconditional, and because no gate reaches them `activateEntries` skips them
  // and the `active: true` default stands unexamined — a parse decision in a module whose
  // whole premise is that activity is the browser's to decide. Seeding the walk puts the
  // sheet's condition into the same carrier stack a `@media` rule builds, so the condition
  // that was false at capture suppresses its rule and the one that was true survives a
  // resize, from one repair rather than two.
  //
  // The inherited state is what makes the seed compose rather than replace: a declaration
  // applies only when the medium matches on every link along the path the sheet was reached
  // by, so an imported sheet's condition nests inside its importer's rather than replacing
  // it. At the top level there is no path yet and the defaults are what a document sheet
  // gets.
  const sheetRules = (rules, condition, media = null, gates = [], carriers = [], seen = new Set()) =>
    condition
      ? flattenRules(
          rules,
          media ? `(${media}) and (${condition})` : condition,
          gates,
          carriers.concat(`@media ${condition}`),
          seen
        )
      : flattenRules(rules, media, gates, carriers, seen);
  // The base every relative reference in a sheet resolves against. CSS 2.1 §4.3.4: "For CSS
  // style sheets, the base URI is that of the style sheet, not that of the source document."
  // A sheet with no location — an inline `<style>`, a document-written one, a constructed one
  // — reports a null `href` and its location *is* the document's, so the document base is the
  // default rather than a case beside it.
  const sheetBase = sheet => (sheet && sheet.href) || document.baseURI;
  // Stamped on the way out of a sheet rather than threaded on the way in, because the base is
  // a property of the sheet and not of the walk's position inside it. An entry that already
  // carries one came from a sheet nested deeper — `@import` resolves innermost-first — and the
  // sheet that produced a rule is the only one entitled to say where its references point.
  const stampBase = (entries, base) => {
    for (const entry of entries) if (!entry.base) entry.base = base;
    return entries;
  };
  // Entering a sheet, however the walk found it. A sheet listed by the document and one
  // hanging off a `CSSImportRule` differ only in how they were reached, so they arrive here
  // together and nothing can be true of one and not the other — which is the whole defect
  // this replaced: `@import` was discarded downstream on the premise that the capture had
  // already walked the sheet it names, and nothing had.
  const pendingSheets = new Map();
  let unreadableSheets = 0;
  const enterSheet = (sheet, media = null, gates = [], carriers = [], seen = new Set()) => {
    // A null sheet is an answer, not a failure: a `supports()` condition that does not match
    // forbids the user agent to fetch the sheet at all. Nothing was loaded, so nothing is
    // owed. Cycles are bounded on sheet identity rather than address, because each link is
    // an independent sheet and two imports of one file must both be walked.
    if (!sheet || seen.has(sheet)) return [];
    seen.add(sheet);
    // `CSSImportRule.media` is defined to be the imported sheet's own `media`, so an
    // import's trailing query arrives here without its prelude ever being parsed.
    const condition = ((sheet.media && sheet.media.mediaText) || '').trim();
    // Recorded before the read and cleared after it, keyed by the URL that is also the only
    // identity the text fallback carries. What remains is exactly the set of sheets still
    // owed their rules, which is why registering here rather than at the caller is what lets
    // the fallback recover a cross-origin imported sheet — the shape most `@import`s on the
    // web have, and the one that carries `@font-face`.
    if (sheet.href) pendingSheets.set(sheet.href, condition);
    let rules;
    try { rules = sheet.cssRules; } catch { unreadableSheets++; return []; }
    try {
      const entries = sheetRules(rules, condition, media, gates, carriers, seen);
      pendingSheets.delete(sheet.href);
      return stampBase(entries, sheetBase(sheet));
    } catch { unreadableSheets++; return []; }
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
