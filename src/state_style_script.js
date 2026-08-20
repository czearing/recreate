__RULE_ACTIVATION__
__SHORTHAND_EXPANSION__
  const cssRules = [], cssRuleKeys = new Set(), stateStyles = [], stateStyleKeys = new Set();
  /* The declarations a state rule carries, with every reference resolved against the element
     the rule styles — because the recreation's own classes need not inherit the tokens the
     source page declared.

     The block's own text is the whole answer. CSSOM serialises a declaration block as the
     author wrote it, which is also the shortest true spelling of it: a shorthand stays one
     rather than becoming the nine longhands it happens to set.

     Enumerating longhands instead loses whole declarations. A shorthand whose value holds
     `var()` is stored as a pending-substitution value on every longhand it sets, and each of
     those serialises as the empty string — measured in Edge, `outline: var(--w) solid
     var(--c)` iterates outline-color, -style and -width with every one reading empty, while
     the authored text stays on the shorthand. Rescuing those by listing shorthand names
     answers for the families somebody listed and withholds the rest, which is why the
     keyboard focus ring of a control whose outline is tokenised reached no record at all. */
  const resolveVariables = (style, element) => {
    const computed = getComputedStyle(element);
    const substitute = value => {
      for (let index = value.indexOf('var('); index >= 0;) {
        const close = /[\w-]/.test(value[index - 1] || '') ? -1 : closingParen(value, index + 3);
        if (close < 0) { index = value.indexOf('var(', index + 4); continue; }
        const inner = value.slice(index + 4, close);
        const comma = topLevelComma(inner);
        const name = (comma < 0 ? inner : inner.slice(0, comma)).trim();
        const replacement =
          computed.getPropertyValue(name).trim() || (comma < 0 ? '' : inner.slice(comma + 1).trim());
        value = value.slice(0, index) + replacement + value.slice(close + 1);
        index = value.indexOf('var(', index + replacement.length);
      }
      return value;
    };
    let resolved = (style.cssText || '').trim();
    for (let pass = 0; pass < 5 && resolved.includes('var('); pass++) resolved = substitute(resolved);
    return resolved;
  };
  // Which states a record can replay. The reader reduces a selector against every state a
  // page can enter, because a query has to match the page as it rests; this narrower set is
  // the one an emitted rule can name.
  const REPLAYABLE_STATE = /^:(hover|focus-visible|focus-within|focus|active)\b/;
  const replayable = states => states.filter(state => REPLAYABLE_STATE.test(state));
  // The element holding the state, given the element the rule styles. Every answer comes from
  // the engine, and the three cases are the three shapes the grammar has: the holder is
  // inside the subject, the holder *is* the subject, or a join separates them. For the join
  // the subject is pinned with an attribute and the authored text is inverted with `:has()`,
  // whose argument is a relative selector anchored at the element it is attached to — so one
  // query answers `~`, `+`, `>`, descendant and any chain of them, and nothing here names a
  // combinator. `closest` is deliberately absent: it walks one axis, so using it to find the
  // holder silently made "not an ancestor" mean "no holder at all".
  const HOLDER_MARK = 'data-recreate-holder';
  const stateHolder = (element, relation) => {
    try {
      if (relation.contained) return element.querySelector(relation.holder);
      if (!relation.rest) return element;
      element.setAttribute(HOLDER_MARK, '');
      try {
        return document.querySelector(
          `${relation.holder}:has(${relation.rest}[${HOLDER_MARK}])`
        );
      } finally {
        element.removeAttribute(HOLDER_MARK);
      }
    } catch (unmatchable) {
      return null;
    }
  };
  const captureStateStyles = (rule, media) => {
    const reduced = media?.includes('prefers-reduced-motion') || false;
    for (const member of selectorMembers(subjectOf(rule))) {
      const box = generatedBoxOf(member);
      const pseudoElement = box ? box.suffix : '';
      const relation = stateRelation(box ? box.subject + box.tail : member);
      const states = replayable(relation.states);
      const tailStates = replayable(relation.tailStates);
      if (!states.length && !reduced) continue;
      if (!relation.query) continue;
      try {
        for (const element of document.querySelectorAll(relation.query)) {
          const owner = stateHolder(element, relation);
          // A contained holder is what the rule is about, so an element without one is not
          // one this rule styles. An ancestor holder that resolves to the element itself is
          // the ordinary case of a state authored on the subject.
          if (relation.contained && !owner) continue;
          const scoped = owner && owner !== element;
          const captured = {
            target: pathOf(element),
            scope: scoped ? pathOf(owner) : null,
            relation: scoped ? relation.axis : 'ancestor',
            pseudo: states.length || pseudoElement
              ? `${scoped ? states[0] : states.join('')}${scoped ? '' : pseudoElement}`
              : null,
            target_pseudo: scoped && (tailStates.length || pseudoElement)
              ? `${tailStates.join('')}${pseudoElement}`
              : null,
            media,
            declarations: resolveVariables(rule.style, element)
          };
          const key = JSON.stringify(captured);
          if (!stateStyleKeys.has(key)) {
            stateStyleKeys.add(key);
            stateStyles.push(captured);
          }
        }
      } catch {}
    }
  };
  // Authored rules are a set, not a cascade log. Two identical rule texts cannot disagree,
  // so recording a text once is the whole of the information either copy carries — except
  // where they came from sheets in different directories, which makes their relative
  // references name different files. The base is therefore part of the record's identity.
  const recordRule = (text, base) => {
    const key = `${base}\u0000${text}`;
    if (cssRuleKeys.has(key)) return;
    cssRuleKeys.add(key);
    cssRules.push({ text, base });
  };
  const emitEntries = entries => {
    for (const { rule, media, active, carriers, base } of entries) {
      // A style rule serialises its nested children inside its own block, so a child recorded
      // in its own right would publish the same declarations a second time.
      if (active && !nestingParent(rule)) {
        // Rebuilding the carrier stack before recording is also what keys the recorded set:
        // two identical declarations in different layers are different declarations, and
        // deduplicating their bare text would collapse them past reconstruction.
        recordRule(
          carriers.reduceRight((inner, prelude) => `${prelude}{${inner}}`, rule.cssText),
          base
        );
      }
      // A state rule is recorded whatever its condition, because the state it describes is
      // entered later, under conditions that need not be the ones in force now.
      if (rule.style && subjectOf(rule)) {
        // Recorded from the same rule object that produced the text above, so the division
        // and the text it divides cannot come from different parses of the same sheet.
        recordShorthandBlock(rule.style, base);
        captureStateStyles(rule, media);
      }
    }
  };
  // `document.styleSheets` excludes constructed sheets adopted by the document or by a
  // shadow root, and `cssRules` throws SecurityError on any cross-origin sheet served
  // without CORS headers. Both cases previously vanished into an empty catch, leaving no
  // authored rules and a page rebuilt entirely from sampled pixels. Unreadable sheets are
  // counted and their text is supplied by the caller, which reads it through the browser's
  // own CSSOM where CORS does not apply.
  const shadowSheets = [];
  for (const scope of treeScopes()) {
    if (scope === document) continue;
    shadowSheets.push(
      ...Array.from(scope.styleSheets || []),
      ...Array.from(scope.adoptedStyleSheets || [])
    );
  }
  const ruleEntries = [];
  // Every collection a sheet can be *listed* in. A sheet reached through `@import` is in
  // none of them, and is entered by the walk itself rather than named here.
  const allSheets = [
    ...Array.from(document.styleSheets),
    ...Array.from(document.adoptedStyleSheets || []),
    ...shadowSheets
  ];
  for (const sheet of allSheets) ruleEntries.push(...enterSheet(sheet));
  // The fallback recovers sheets the page could not read, so a sheet already walked above
  // must not be walked again from its text. `recordRule` keys on exact rule text, and a
  // second copy carrying a different condition is a different key, so re-walking would
  // re-emit every conditioned rule unconditioned beside its correct form and no
  // deduplication could collapse the pair. Only a sheet with its own URL can be unreadable;
  // a `<style>` element, a document-written one and a constructed one all serialise under
  // the document's own address, are absent from this map, and are readable by construction.
  for (const { text, href } of authoredSheetTexts) {
    if (!pendingSheets.has(href)) continue;
    try {
      const sheet = new CSSStyleSheet();
      sheet.replaceSync(text);
      ruleEntries.push(...stampBase(sheetRules(sheet.cssRules, pendingSheets.get(href)), href));
      pendingSheets.delete(href);
      if (unreadableSheets > 0) unreadableSheets--;
    } catch {}
  }
  // Activation is resolved for every collected rule at once, so the whole walk costs one
  // style recalculation rather than one per conditional rule.
  emitEntries(activateEntries(ruleEntries));
