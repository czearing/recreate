use crate::style_contract;

const SOURCE: &str = r#"
(async () => {
  const props = [__STYLE_PROPERTIES__];
  const directionalBorders = [__DIRECTIONAL_BORDERS__];
  const styleMap = style => {
    const values = Object.fromEntries(props.map(property =>
      [property, style.getPropertyValue(property)]
    ));
    if (!values.border) {
      for (const property of directionalBorders) {
        values[property] = style.getPropertyValue(property);
      }
    }
    return values;
  };
  const pathOf = element => {
    if (element === document.documentElement) return 'html';
    const parts = [];
    for (let node = element; node && node !== document.documentElement; node = node.parentElement) {
      const peers = node.parentElement
        ? [...node.parentElement.children].filter(child => child.tagName === node.tagName)
        : [node];
      parts.push(`${node.tagName.toLowerCase()}:nth-of-type(${peers.indexOf(node) + 1})`);
    }
    return `html>${parts.reverse().join('>')}`;
  };
  const visible = element => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 &&
      style.display !== 'none' && style.visibility !== 'hidden' &&
      Number(style.opacity || 1) > 0.01;
  };
  const selected = new Set([document.documentElement, document.body]);
  const roots = [...document.querySelectorAll(
    '[role="dialog"],[role="listbox"],[role="menu"]'
  )].filter(visible);
  for (const button of document.querySelectorAll('button,[role="button"]')) {
    if (!/^(pin|delete|duplicate)$/i.test((button.innerText || '').trim())) continue;
    for (let parent = button.parentElement; parent; parent = parent.parentElement) {
      const position = getComputedStyle(parent).position;
      if (position === 'fixed' || position === 'absolute') {
        roots.push(parent);
        break;
      }
    }
  }
  if (document.activeElement instanceof Element &&
      document.activeElement !== document.body) {
    let root = document.activeElement;
    for (let depth = 0; depth < 3 && root.parentElement; depth++) {
      root = root.parentElement;
    }
    roots.push(root);
  }
  for (const animation of document.getAnimations({ subtree: true })) {
    if (animation.effect?.target instanceof Element) roots.push(animation.effect.target);
  }
  for (const root of roots) {
    selected.add(root);
    root.querySelectorAll('*').forEach(element => selected.add(element));
    for (let parent = root.parentElement; parent; parent = parent.parentElement) {
      selected.add(parent);
    }
  }
  const nodes = [];
  const capture = element => {
    const path = pathOf(element);
    const rect = element.getBoundingClientRect();
    nodes.push({
      path,
      parent: element.parentElement ? pathOf(element.parentElement) : null,
      tag: element.tagName.toLowerCase(),
      text: '',
      attributes: Object.fromEntries([...element.attributes]
        .filter(attribute => !attribute.name.startsWith('on') &&
          !['style','nonce','integrity'].includes(attribute.name))
        .map(attribute => [attribute.name, attribute.value])),
      rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
      style: styleMap(getComputedStyle(element)),
      before: null,
      after: null
    });
    let textIndex = 0;
    for (const child of element.childNodes) {
      if (child.nodeType !== Node.TEXT_NODE || !child.textContent.trim()) continue;
      const range = document.createRange();
      range.selectNodeContents(child);
      const value = range.getBoundingClientRect();
      textIndex++;
      nodes.push({
        path: `${path}>#text(${textIndex})`,
        parent: path,
        tag: '#text',
        text: child.textContent.replace(/\s+/g, ' '),
        attributes: {},
        rect: { x: value.x, y: value.y, width: value.width, height: value.height },
        style: styleMap(getComputedStyle(element)),
        before: null,
        after: null
      });
    }
  };
  for (const element of document.querySelectorAll('*')) {
    if (selected.has(element)) capture(element);
  }
  return JSON.stringify({
    url: location.href,
    title: document.title,
    nodes,
    animations: [],
    state_styles: [],
    attribute_sequences: [],
    css_rules: [],
    asset_urls: [],
    asset_data: {}
  });
})()
"#;

pub fn source() -> String {
    SOURCE
        .replace("__STYLE_PROPERTIES__", style_contract::PROPERTIES)
        .replace(
            "__DIRECTIONAL_BORDERS__",
            style_contract::DIRECTIONAL_BORDERS,
        )
}
