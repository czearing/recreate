pub const SOURCE: &str = r#"
__RULE_ACTIVATION__
  const cssRules = [], cssRuleKeys = new Set(), stateStyles = [], stateStyleKeys = new Set();
  const stateShorthands = [
    'animation','background','border','border-color','border-radius','border-style',
    'border-width','flex','font','gap','grid','inset','margin','mask','padding',
    'transition'
  ];
  const resolveVariables = (style, element) => {
    const computed = getComputedStyle(element);
    const resolveValue = value => {
      let resolved = value;
      for (let pass = 0; pass < 5 && resolved.includes('var('); pass++) {
        resolved = resolved.replace(
          /var\((--[\w-]+)(?:,\s*([^)]*))?\)/g,
          (_, name, fallback = '') => computed.getPropertyValue(name).trim() || fallback.trim()
        );
      }
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
  // Authored rules are a set, not a cascade log: the caller re-supplies sheet text for
  // sheets the page could not read, and which ones failed cannot be matched back to that
  // text, so the same sheet is walked twice. Two identical rule texts cannot disagree, so
  // recording a text once is the whole of the information either copy carries.
  const recordRule = text => {
    if (cssRuleKeys.has(text)) return;
    cssRuleKeys.add(text);
    cssRules.push(text);
  };
  const emitEntries = entries => {
    for (const { rule, media, active, carriers } of entries) {
      if (active) {
        // Rebuilding the carrier stack before recording is also what keys the recorded set:
        // two identical declarations in different layers are different declarations, and
        // deduplicating their bare text would collapse them past reconstruction.
        recordRule(carriers.reduceRight(
          (inner, prelude) => `${prelude}{${inner}}`,
          rule.cssText
        ));
      }
      // A state rule is recorded whatever its condition, because the state it describes is
      // entered later, under conditions that need not be the ones in force now.
      if (rule.selectorText && rule.style) captureStateStyles(rule, media);
    }
  };
  // `document.styleSheets` excludes constructed sheets adopted by the document or by a
  // shadow root, and `cssRules` throws SecurityError on any cross-origin sheet served
  // without CORS headers. Both cases previously vanished into an empty catch, leaving no
  // authored rules and a page rebuilt entirely from sampled pixels. Unreadable sheets are
  // counted and their text is supplied by the caller, which reads it through the browser's
  // own CSSOM where CORS does not apply.
  const shadowSheets = [];
  const collectShadowSheets = root => {
    for (const element of root.querySelectorAll('*')) {
      const shadow = element.shadowRoot;
      if (!shadow) continue;
      for (const sheet of shadow.adoptedStyleSheets || []) shadowSheets.push(sheet);
      for (const style of shadow.querySelectorAll('style')) {
        if (style.sheet) shadowSheets.push(style.sheet);
      }
      collectShadowSheets(shadow);
    }
  };
  try { collectShadowSheets(document); } catch {}
  let unreadableSheets = 0;
  const ruleEntries = [];
  const allSheets = [
    ...Array.from(document.styleSheets),
    ...Array.from(document.adoptedStyleSheets || []),
    ...shadowSheets
  ];
  for (const sheet of allSheets) {
    let rules = null;
    try { rules = sheet.cssRules; } catch { unreadableSheets++; continue; }
    try { ruleEntries.push(...flattenRules(rules)); } catch { unreadableSheets++; }
  }
  for (const text of authoredSheetTexts) {
    try {
      const sheet = new CSSStyleSheet();
      sheet.replaceSync(text);
      ruleEntries.push(...flattenRules(sheet.cssRules));
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
