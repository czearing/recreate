export const hoverRuleExpression = `(() => {
  const root = window.__recreateHoverRoot;
  const rules = [];
  const visit = list => {
    for (const rule of list || []) {
      if (rule.cssRules) visit(rule.cssRules);
      const selectorText = rule.selectorText || '';
      if (!selectorText.includes(':hover')) continue;
      const matches = selectorText.split(',').some(selector => {
        const anchor = selector.slice(0, selector.indexOf(':hover')).trim();
        try { return root.matches(anchor); } catch { return false; }
      });
      if (matches) rules.push(rule.cssText || '');
    }
  };
  const roots = [document];
  const sheets = [
    ...document.styleSheets,
    ...document.adoptedStyleSheets,
  ];
  while (roots.length) {
    const currentRoot = roots.shift();
    if (currentRoot instanceof ShadowRoot) {
      sheets.push(...currentRoot.adoptedStyleSheets);
      for (const style of currentRoot.querySelectorAll('style')) {
        if (style.sheet) sheets.push(style.sheet);
      }
    }
    for (const element of currentRoot.querySelectorAll('*')) {
      if (element.shadowRoot) roots.push(element.shadowRoot);
    }
  }
  for (const sheet of sheets) {
    try { visit(sheet.cssRules); } catch {}
  }
  return rules.slice(0, 50);
})()`;
