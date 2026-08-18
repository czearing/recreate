pub const SOURCE: &str = r#"
__RULE_ACTIVATION__
__SHORTHAND_EXPANSION__
  const cssRules = [], cssRuleKeys = new Set(), stateStyles = [], stateStyleKeys = new Set();
  const stateShorthands = [
    'animation','background','border','border-color','border-radius','border-style',
    'border-width','flex','font','gap','grid','inset','margin','mask','padding',
    'transition'
  ];
  const scanValue = (text, from, accept) => {
    let depth = 0;
    for (let index = from; index < text.length; index++) {
      const char = text[index];
      if (char === '\\') { index++; continue; }
      if (char === '"' || char === "'") {
        for (index++; index < text.length && text[index] !== char; index++) {
          if (text[index] === '\\') index++;
        }
        continue;
      }
      if (char === '(') depth++;
      else if (char === ')') depth--;
      if (accept(char, depth)) return index;
    }
    return -1;
  };
  const closingParen = (text, open) => scanValue(text, open, (char, depth) => char === ')' && !depth);
  const topLevelComma = text => scanValue(text, 0, (char, depth) => char === ',' && !depth);
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
    const resolveValue = value => {
      let resolved = value;
      for (let pass = 0; pass < 5 && resolved.includes('var('); pass++) resolved = substitute(resolved);
      return resolved;
    };
    const names = new Set(Array.from(style));
    for (const name of stateShorthands) {
      if (style.getPropertyValue(name).trim()) names.add(name);
    }
    return Array.from(names).map(name => {
      const priority = style.getPropertyPriority(name);
      const value = resolveValue(style.getPropertyValue(name));
      if (!value) return '';
      return `${name}: ${value}${priority ? ` !${priority}` : ''};`;
    }).filter(Boolean).join(' ');
  };
  const captureStateStyles = (rule, media) => {
    const reduced = media?.includes('prefers-reduced-motion') || false;
    for (const selector of rule.selectorText.split(',')) {
      const states = Array.from(selector.matchAll(dynamicStatePattern), match => match[0]);
      const tail = selector.trim().split(/[\s>+~]+/).pop() || '';
      const tailStates = Array.from(tail.matchAll(dynamicStatePattern), match => match[0]);
      if (!states.length && !reduced) continue;
      const base = selector.replace(dynamicStatePattern, '').trim();
      const pseudoElement = base.match(/::[\w-]+$/)?.[0] || '';
      const query = base.slice(0, base.length - pseudoElement.length);
      if (!query) continue;
      try {
        for (const element of document.querySelectorAll(query)) {
          const stateIndex = selector.search(/:(hover|focus-visible|focus-within|focus|active)\b/);
          const ownerQuery = stateIndex >= 0 ? selector.slice(0, stateIndex).trim() : '';
          let owner = null;
          if (ownerQuery) {
            try { owner = element.closest(ownerQuery); } catch {}
          }
          const scoped = owner && owner !== element;
          const captured = {
            target: pathOf(element),
            scope: scoped ? pathOf(owner) : null,
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
      if (active) {
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
      if (rule.selectorText && rule.style) {
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
"#;

#[cfg(test)]
mod tests {
    #[test]
    fn resolves_custom_properties_in_dynamic_state_rules() {
        assert!(super::SOURCE.contains("computed.getPropertyValue(name)"));
        assert!(super::SOURCE.contains("declarations: resolveVariables(rule.style, element)"));
        assert!(super::SOURCE.contains("'background'"));
        assert!(super::SOURCE.contains("if (!value) return ''"));
    }
}
