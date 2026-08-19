
(async () => {
  const authoredSheetTexts = __AUTHORED_SHEETS__;
  const ignored = new Set(['SCRIPT','NOSCRIPT']);
__STYLE_BASELINE__
__ASSET_ATTRIBUTES__
  const skipped = element =>
    ignored.has(element.tagName) || element.hasAttribute('data-recreate-startup');
  const isBlockingOverlay = __BLOCKING_OVERLAY__;
  const resumeMotion = __MOTION_POLICY__(() => measureBaselines(document.documentElement, skipped));
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
__NODE_PATH__
  // The space a classic scrollbar took out of the content box. `offsetWidth - clientWidth`
  // is the border box minus the padding box, so the two border widths are the rest of it.
  // Read here rather than derived later because `style` is pruned to the declarations that
  // differ from the element's baseline, which discards user-agent padding and border.
  // `clientWidth` is defined as zero when the element has no padding box to report - an
  // inline box, or no layout box at all - and there is no gutter inside a box that does not
  // exist, so that is the condition rather than a list of elements to skip.
  const scrollbarGutter = (element, style) => {
    if (!element.clientWidth) return 0;
    const gutter = element.offsetWidth - element.clientWidth
      - (parseFloat(style.borderLeftWidth) || 0)
      - (parseFloat(style.borderRightWidth) || 0);
    return gutter > 0 ? gutter : 0;
  };
  const nodes = [];
  // The element each record came from, so a stage that must ask the engine a second
  // question can ask it of the element and answer it onto the record.
  const elementNodes = [];
  const dom = {};
  const recordDom = (path, element, style, overrides = {}) => {
    const root = element.getRootNode();
    dom[path] = {
      namespace: element.namespaceURI || '',
      node_type: element.nodeType,
      tree_scope: root instanceof ShadowRoot ? shadowPath(root) : 'document',
      physical_parent: holderPath(element),
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
    const attributes = recreateAttributes(element, path);
    const record = {
      path,
      parent: holderPath(element),
      tag: element.tagName.toLowerCase(),
      text: '',
      attributes,
      control_state: recreateControlState(element),
      disabled: element.matches(':disabled'),
      modal: element.matches(':modal'),
      blocking_overlay: isBlockingOverlay(element),
      rtl: computedStyle.direction === 'rtl',
      writing_mode: computedStyle.writingMode,
      rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
      scrollbar_gutter: scrollbarGutter(element, computedStyle),
      style: authoredStyles(styleMap(computedStyle), baselineOf(element)),
      pseudos: recreatePseudos(element)
    };
    nodes.push(record);
    elementNodes.push([element, record]);
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
          style: authoredStyles(styleMap(getComputedStyle(element)), baselineOf(element)),
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
  // The reading is over, so the page may move again — and must, because the next reader's
  // whole subject is the motion this one was holding out.
  resumeMotion();
  const liveAnimations = document.getAnimations({ subtree: true }).map(animation => {
    const timing = animation.effect?.getTiming?.() || {};
    return {
      target: animation.effect?.target ? pathOf(animation.effect.target) : '',
      // A CSSAnimation names the @keyframes block the author wrote. A script-driven
      // animation has no such name, and that absence is what says it must be rebuilt from
      // samples rather than deferred to.
      name: animation.animationName || '',
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
__CONDITION_WITHDRAWAL__
__ATTRIBUTE_SEQUENCE_CAPTURE__
  const { texts: cssRuleTexts, urls: assets, shorthands } =
    recreateCssAssets(nodes, cssRules, shorthandBlocks.values());
__ASSET_CAPTURE__
  return JSON.stringify({
    url: location.href,
    title: document.title,
    nodes,
    dom,
    capture_blockers: (unreadableSheets > 0
      ? [`${unreadableSheets} stylesheet(s) could not be read; their authored rules are missing`]
      : [])
      .concat(window.__recreateUnsettled
        ? ['page never reported itself settled; it was read at the stability ceiling']
        : [])
      .concat(recreateSurfaceBlockers())
      .concat(recreatePseudoBlockers()),
    animations,
    state_styles: stateStyles,
    attribute_sequences: attributeSequences,
    css_rules: cssRuleTexts,
    css_shorthands: shorthands,
    asset_urls: Array.from(assets),
    asset_data: assetData
  });
})()
