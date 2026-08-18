(async () => {
__STYLE_BASELINE__
__ASSET_ATTRIBUTES__
  const skipped = element => false;
  measureBaselines(document.documentElement, skipped);
__NODE_PATH__
  const visible = element => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 &&
      style.display !== 'none' && style.visibility !== 'hidden' &&
      Number(style.opacity || 1) > 0.01;
  };
__SELECTION__
  const nodes = [];
  const dom = {};
  const capture = element => {
    if (element.matches('script,noscript,[data-recreate-startup],.recreateAnchoredSurface')) return;
    const path = pathOf(element);
    const rect = element.getBoundingClientRect();
    const computedStyle = getComputedStyle(element);
    dom[path] = {
      namespace: element.namespaceURI || '',
      node_type: element.nodeType,
      tree_scope: 'document',
      physical_parent: holderPath(element),
      assigned_slot: element.assignedSlot ? pathOf(element.assignedSlot) : null,
      shadow_root_mode: element.shadowRoot?.mode || null,
      client_rects: [...element.getClientRects()].map(value => ({
        x: value.x, y: value.y, width: value.width, height: value.height
      })),
      scroll_left: element.scrollLeft,
      scroll_top: element.scrollTop,
      scroll_width: element.scrollWidth,
      scroll_height: element.scrollHeight,
      client_width: element.clientWidth,
      client_height: element.clientHeight,
      computed_style_properties: [],
      computed_style_dictionary: [],
      computed_style_values: [],
      custom_properties: {}
    };
    nodes.push({
      path,
      parent: holderPath(element),
      tag: element.tagName.toLowerCase(),
      text: '',
      attributes: recreateAttributes(element, path),
      control_state: recreateControlState(element),
      rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
      style: authoredStyles(styleMap(computedStyle), baselineOf(element)),
      disabled: element.matches(':disabled'),
      modal: element.matches(':modal'),
      rtl: computedStyle.direction === 'rtl',
      writing_mode: computedStyle.writingMode,
      pseudos: {}
    });
    let textIndex = 0;
    for (const child of element.childNodes) {
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
        style: authoredStyles(styleMap(getComputedStyle(element)), baselineOf(element)),
        pseudos: {}
      });
    }
  };
  for (const element of document.querySelectorAll('*')) {
    if (selected.has(element)) capture(element);
  }
  const assets = recreateAssetUrls(nodes, []);
__ASSET_CAPTURE__
  return JSON.stringify({
    url: location.href,
    title: document.title,
    nodes,
    dom,
    animations: [],
    state_styles: [],
    attribute_sequences: [],
    css_rules: [],
    asset_urls: Array.from(assets),
    asset_data: assetData
  });
})()
