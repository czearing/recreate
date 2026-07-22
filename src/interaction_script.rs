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
  const pathCache = new WeakMap([[document.documentElement, 'html']]);
  const siblingIndexes = new WeakMap();
  const pathOf = element => {
    const cached = pathCache.get(element);
    if (cached) return cached;
    const parent = element.parentElement;
    let indexes = siblingIndexes.get(parent);
    if (!indexes) {
      indexes = new WeakMap();
      const counts = new Map();
      for (const child of parent.children) {
        const count = (counts.get(child.tagName) || 0) + 1;
        counts.set(child.tagName, count);
        indexes.set(child, count);
      }
      siblingIndexes.set(parent, indexes);
    }
    const path = `${pathOf(parent)}>${element.tagName.toLowerCase()}:nth-of-type(${indexes.get(element)})`;
    pathCache.set(element, path);
    return path;
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
  for (const portal of document.querySelectorAll('body>[data-portal-node]')) {
    if (visible(portal) || [...portal.querySelectorAll('*')].some(visible)) roots.push(portal);
  }
  for (const element of document.querySelectorAll('body *')) {
    const rect = element.getBoundingClientRect();
    if (visible(element) && getComputedStyle(element).position === 'fixed' &&
        rect.width * rect.height >= 400) roots.push(element);
  }
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
  for (const root of roots) {
    selected.add(root);
    root.querySelectorAll('*').forEach(element => selected.add(element));
    for (let parent = root.parentElement; parent; parent = parent.parentElement) {
      selected.add(parent);
    }
  }
  const nodes = [];
  const capture = element => {
    if (element.matches('script,noscript,[data-recreate-startup],.recreateAnchoredSurface')) return;
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
    const textChildren = element.matches('textarea,input') && element.value.trim()
      ? [document.createTextNode(element.value)]
      : element.childNodes;
    for (const child of textChildren) {
      if (child.nodeType !== Node.TEXT_NODE || !child.textContent.trim()) continue;
      const range = document.createRange();
      if (child.parentNode) range.selectNodeContents(child);
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
  const assets = new Set();
  for (const element of selected) {
    if (element.matches?.('img,video,source')) {
      const url = element.currentSrc || element.src;
      if (url) assets.add(url);
    }
  }
  for (const node of nodes) {
    for (const match of node.style['background-image'].matchAll(/url\(["']?([^"')]+)["']?\)/g)) {
      assets.add(new URL(match[1], location.href).href);
    }
  }
__ASSET_CAPTURE__
  return JSON.stringify({
    url: location.href,
    title: document.title,
    nodes,
    animations: [],
    state_styles: [],
    attribute_sequences: [],
    css_rules: [],
    asset_urls: Array.from(assets),
    asset_data: assetData
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
        .replace("__ASSET_CAPTURE__", crate::asset_script::SOURCE)
}

#[cfg(test)]
mod tests {
    #[test]
    fn captures_portal_and_fixed_menu_roots() {
        assert!(super::SOURCE.contains("body>[data-portal-node]"));
        assert!(super::SOURCE.contains("position === 'fixed'"));
    }

    #[test]
    fn excludes_non_rendered_runtime_nodes() {
        assert!(
            super::SOURCE
                .contains("script,noscript,[data-recreate-startup],.recreateAnchoredSurface")
        );
    }

    #[test]
    fn selection_does_not_depend_on_generated_animations() {
        assert!(!super::SOURCE.contains("document.getAnimations"));
    }

    #[test]
    fn captures_selected_surface_assets() {
        let source = super::source();
        assert!(source.contains("const assetData = {}"));
        assert!(source.contains("asset_data: assetData"));
    }

    #[test]
    fn canonicalizes_control_values_as_text_evidence() {
        assert!(super::SOURCE.contains("element.matches('textarea,input')"));
        assert!(super::SOURCE.contains("document.createTextNode(element.value)"));
    }
}
