use crate::{asset_script, style_contract};
const CAPTURE: &str = r#"
(async () => {
  const props = [__STYLE_PROPERTIES__];
  const ignored = new Set(['SCRIPT','NOSCRIPT']), directionalBorders = [__DIRECTIONAL_BORDERS__];
  const styleMap = style => { const values = Object.fromEntries(props.map(p => [p, style.getPropertyValue(p)])); if (!values.border) for (const property of directionalBorders) values[property] = style.getPropertyValue(property); return values; };
  const computedStyles = new WeakMap(), computedStylePropertySet = new Set();
  const scan = element => {
    if (ignored.has(element.tagName) || element.hasAttribute('data-recreate-startup')) return;
    const style = getComputedStyle(element);
    computedStyles.set(element, style);
    for (const property of style) computedStylePropertySet.add(property);
    for (const child of element.children) scan(child);
    if (element.shadowRoot) for (const child of element.shadowRoot.children) scan(child);
  };
  scan(document.documentElement);
  const computedStyleProperties = Array.from(computedStylePropertySet).sort();
  const computedStyleDictionary = [], computedStyleIds = new Map();
  const computedStyleValue = value => {
    if (computedStyleIds.has(value)) return computedStyleIds.get(value);
    const index = computedStyleDictionary.length;
    computedStyleDictionary.push(value);
    computedStyleIds.set(value, index);
    return index;
  };
  const computedStyleValues = style => computedStyleProperties
    .map(property => computedStyleValue(style.getPropertyValue(property)));
  const pathCache = new WeakMap([[document.documentElement, 'html']]);
  const siblingIndexes = new WeakMap();
  const siblingIndex = element => {
    const root = element.parentElement || element.getRootNode();
    let indexes = siblingIndexes.get(root);
    if (!indexes) {
      indexes = new WeakMap();
      const counts = new Map();
      for (const child of root.children || []) {
        const count = (counts.get(child.tagName) || 0) + 1;
        counts.set(child.tagName, count);
        indexes.set(child, count);
      }
      siblingIndexes.set(root, indexes);
    }
    return indexes.get(element) || 1;
  };
  const shadowPath = root => `${pathOf(root.host)}>::shadow-root(${root.mode})`;
  const pathOf = element => {
    const cached = pathCache.get(element);
    if (cached) return cached;
    const root = element.getRootNode();
    const parent = element.parentElement
      ? pathOf(element.parentElement)
      : root instanceof ShadowRoot ? shadowPath(root) : 'html';
    const path = `${parent}>${element.tagName.toLowerCase()}:nth-of-type(${siblingIndex(element)})`;
    pathCache.set(element, path);
    return path;
  };
  const pseudo = (element, name) => {
    const style = getComputedStyle(element, name);
    const content = style.content;
    return content && content !== 'none'
      ? { content, style: styleMap(style) }
      : null;
  };
  const nodes = [];
  const dom = {};
  const recordDom = (path, element, style, overrides = {}) => {
    const root = element.getRootNode();
    dom[path] = {
      namespace: element.namespaceURI || '',
      node_type: element.nodeType,
      tree_scope: root instanceof ShadowRoot ? shadowPath(root) : 'document',
      physical_parent: element.parentElement
        ? pathOf(element.parentElement)
        : root instanceof ShadowRoot ? shadowPath(root) : null,
      assigned_slot: element.assignedSlot ? pathOf(element.assignedSlot) : null,
      shadow_root_mode: null,
      client_rects: Array.from(element.getClientRects(), rect => ({
        x: rect.x, y: rect.y, width: rect.width, height: rect.height
      })),
      scroll_left: element.scrollLeft || 0,
      scroll_top: element.scrollTop || 0,
      scroll_width: element.scrollWidth || 0,
      scroll_height: element.scrollHeight || 0,
      client_width: element.clientWidth || 0,
      client_height: element.clientHeight || 0,
      computed_style_properties: path === 'html' ? computedStyleProperties : [],
      computed_style_dictionary: path === 'html' ? computedStyleDictionary : [],
      computed_style_values: computedStyleValues(style),
      custom_properties: {},
      ...overrides
    };
  };
  const walk = element => {
    if (ignored.has(element.tagName) || element.hasAttribute('data-recreate-startup')) return;
    const path = pathOf(element);
    const rect = element.getBoundingClientRect();
    const computedStyle = computedStyles.get(element) || getComputedStyle(element);
    const attributes = Object.fromEntries(
      Array.from(element.attributes)
        .filter(attribute =>
          !attribute.name.startsWith('on') &&
          !['style','nonce','integrity'].includes(attribute.name)
        )
        .map(attribute => [attribute.name, attribute.value])
    );
    nodes.push({
      path,
      parent: element.parentElement ? pathOf(element.parentElement) : null,
      tag: element.tagName.toLowerCase(),
      text: '',
      attributes,
      rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
      style: styleMap(computedStyle),
      before: pseudo(element, '::before'),
      after: pseudo(element, '::after')
    });
    recordDom(path, element, computedStyle);
    let textIndex = 0;
    for (const child of element.childNodes) {
      if (child.nodeType === Node.ELEMENT_NODE) {
        walk(child);
      } else if (child.nodeType === Node.TEXT_NODE) {
        const rawText = child.textContent || '';
        if (!rawText.trim()) continue;
        const text = rawText.replace(/\s+/g, ' ');
        textIndex++;
        const range = document.createRange();
        range.selectNodeContents(child);
        const textRect = range.getBoundingClientRect();
        nodes.push({
          path: `${path}>#text(${textIndex})`,
          parent: path,
          tag: '#text',
          text,
          attributes: {},
          rect: {
            x: textRect.x, y: textRect.y,
            width: textRect.width, height: textRect.height
          },
          style: styleMap(getComputedStyle(element)),
          before: null,
          after: null
        });
      }
    }
    if (element.shadowRoot) {
      const root = element.shadowRoot;
      const rootPath = shadowPath(root);
      nodes.push({
        path: rootPath,
        parent: path,
        tag: '#shadow-root',
        text: '',
        attributes: {},
        rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
        style: {},
        before: null,
        after: null
      });
      dom[rootPath] = {
        namespace: '',
        node_type: root.nodeType,
        tree_scope: rootPath,
        physical_parent: path,
        assigned_slot: null,
        shadow_root_mode: root.mode,
        client_rects: [],
        scroll_left: 0,
        scroll_top: 0,
        scroll_width: 0,
        scroll_height: 0,
        client_width: 0,
        client_height: 0,
        computed_style_properties: [],
        computed_style_dictionary: [],
        computed_style_values: [],
        custom_properties: {}
      };
      for (const child of root.children) walk(child);
    }
  };
  walk(document.documentElement);
  const liveAnimations = document.getAnimations({ subtree: true }).map(animation => {
    const timing = animation.effect?.getTiming?.() || {};
    return {
      target: animation.effect?.target ? pathOf(animation.effect.target) : '',
      keyframes: animation.effect?.getKeyframes?.() || [],
      timing: {
        ...timing,
        iterations: timing.iterations === Infinity ? 'infinite' : timing.iterations,
        playState: animation.playState,
        playbackRate: animation.playbackRate
      }
    };
  }).filter(animation => animation.target);
  const nodePaths = new Set(nodes.map(node => node.path));
  const meaningfulTransient = animation => {
    if (nodePaths.has(animation.target)) return true;
    const frames = animation.keyframes || [];
    if (frames.length < 2) return false;
    const changed = key => new Set(frames.map(frame => String(frame[key]))).size > 1;
    if (changed('opacity') || changed('transform') || changed('y') || changed('height')) {
      return true;
    }
    const centers = frames.map(frame => Number(frame.x || 0) + Number(frame.width || 0) / 2);
    return Math.max(...centers) - Math.min(...centers) > 1;
  };
  const animations = [
    ...liveAnimations,
    ...(window.__recreateLifecycleAnimations || [])
  ].filter(meaningfulTransient);
__STATE_STYLE_CAPTURE__
__ATTRIBUTE_SEQUENCE_CAPTURE__
  const assets = new Set();
  document.querySelectorAll('img,video,source').forEach(element => {
    const url = element.currentSrc || element.src;
    if (url) assets.add(url);
  });
  for (const node of nodes) {
    const matches = node.style['background-image'].matchAll(/url\(["']?([^"')]+)["']?\)/g);
    for (const match of matches) assets.add(new URL(match[1], location.href).href);
  }
  for (const rule of cssRules) {
    for (const match of rule.matchAll(/url\(["']?([^"')]+)["']?\)/g)) {
      try {
        const url = new URL(match[1], location.href).href;
        if (!url.startsWith('data:')) assets.add(url);
      } catch {}
    }
  }
__ASSET_CAPTURE__
  return JSON.stringify({
    url: location.href,
    title: document.title,
    nodes,
    dom,
    capture_blockers: [],
    animations,
    state_styles: stateStyles,
    attribute_sequences: attributeSequences,
    css_rules: cssRules,
    asset_urls: Array.from(assets),
    asset_data: assetData
  });
})()
"#;

pub fn source() -> String {
    CAPTURE
        .replace("__STYLE_PROPERTIES__", style_contract::PROPERTIES)
        .replace(
            "__DIRECTIONAL_BORDERS__",
            style_contract::DIRECTIONAL_BORDERS,
        )
        .replace("__STATE_STYLE_CAPTURE__", crate::state_style_script::SOURCE)
        .replace(
            "__ATTRIBUTE_SEQUENCE_CAPTURE__",
            crate::attribute_sequence_script::SOURCE,
        )
        .replace("__ASSET_CAPTURE__", asset_script::SOURCE)
}

pub fn source_without_assets() -> String {
    CAPTURE
        .replace("__STYLE_PROPERTIES__", style_contract::PROPERTIES)
        .replace(
            "__DIRECTIONAL_BORDERS__",
            style_contract::DIRECTIONAL_BORDERS,
        )
        .replace("__STATE_STYLE_CAPTURE__", crate::state_style_script::SOURCE)
        .replace(
            "__ATTRIBUTE_SEQUENCE_CAPTURE__",
            crate::attribute_sequence_script::SOURCE,
        )
        .replace("__ASSET_CAPTURE__", "const assetData = {};")
}

#[cfg(test)]
#[path = "page_script_tests.rs"]
mod tests;
