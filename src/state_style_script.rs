pub const SOURCE: &str = r#"
  const cssRules = [], stateStyles = [], stateStyleKeys = new Set();
  const dynamicState = /:(hover|focus-visible|focus-within|focus|active)\b/g;
  const visitRules = (rules, media = null) => {
    for (const rule of Array.from(rules || [])) {
      cssRules.push(rule.cssText);
      const reduced = media?.includes('prefers-reduced-motion') || false;
      if (rule.selectorText && rule.style) {
        for (const selector of rule.selectorText.split(',')) {
          const states = Array.from(selector.matchAll(dynamicState), match => match[0]);
          const tail = selector.trim().split(/[\s>+~]+/).pop() || '';
          const tailStates = Array.from(tail.matchAll(dynamicState), match => match[0]);
          if (!states.length && !reduced) continue;
          const base = selector.replace(dynamicState, '').trim();
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
                declarations: rule.style.cssText
              };
              const key = JSON.stringify(captured);
              if (!stateStyleKeys.has(key)) {
                stateStyleKeys.add(key);
                stateStyles.push(captured);
              }
            }
          } catch {}
        }
      }
      if (rule.cssRules) {
        const nestedMedia = rule.type === CSSRule.MEDIA_RULE
          ? (media ? `(${media}) and (${rule.conditionText})` : rule.conditionText)
          : media;
        visitRules(rule.cssRules, nestedMedia);
      }
    }
  };
  for (const sheet of Array.from(document.styleSheets)) {
    try { visitRules(sheet.cssRules); } catch {}
  }
"#;
