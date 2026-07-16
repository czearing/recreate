pub const CAPTURE: &str = r#"
(async () => {
  const props = [
    'display','position','inset','top','right','bottom','left','box-sizing',
    'width','height','min-width','max-width','min-height','max-height',
    'margin','padding','gap','row-gap','column-gap','flex','flex-grow',
    'flex-shrink','flex-basis','flex-direction','flex-wrap','justify-content',
    'align-items','align-self','order','grid-template-columns',
    'grid-template-rows','grid-auto-flow','overflow','overflow-x','overflow-y',
    'z-index','color','background-color','background-image','background-size',
    'background-position','background-repeat','border','border-radius',
    'box-shadow','opacity','filter','transform','transform-origin',
    'font-family','font-size','font-weight','font-style','line-height',
    'font-stretch','font-kerning','font-feature-settings','font-variation-settings',
    'letter-spacing','text-align','text-transform','text-rendering','white-space','word-break',
    'object-fit','object-position','cursor','pointer-events','transition',
    'animation','mask-image','mask-size','mask-position','mask-repeat',
    'mask-composite','clip-path'
  ];
  const ignored = new Set(['SCRIPT','STYLE','NOSCRIPT','META','LINK','HEAD']);
  const styleMap = style => Object.fromEntries(props.map(p => [p, style.getPropertyValue(p)]));
  const pathOf = element => {
    if (element === document.documentElement) return 'html';
    const parts = [];
    for (let node = element; node && node !== document.documentElement; node = node.parentElement) {
      const tag = node.tagName.toLowerCase();
      const peers = node.parentElement
        ? Array.from(node.parentElement.children).filter(child => child.tagName === node.tagName)
        : [node];
      parts.push(`${tag}:nth-of-type(${peers.indexOf(node) + 1})`);
    }
    return `html>${parts.reverse().join('>')}`;
  };
  const pseudo = (element, name) => {
    const style = getComputedStyle(element, name);
    const content = style.content;
    return content && content !== 'none'
      ? { content, style: styleMap(style) }
      : null;
  };
  const nodes = [];
  const walk = element => {
    if (ignored.has(element.tagName)) return;
    const path = pathOf(element);
    const rect = element.getBoundingClientRect();
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
      style: styleMap(getComputedStyle(element)),
      before: pseudo(element, '::before'),
      after: pseudo(element, '::after')
    });
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
  };
  walk(document.documentElement);
  const liveAnimations = document.getAnimations({ subtree: true }).map(animation => ({
    target: animation.effect?.target ? pathOf(animation.effect.target) : '',
    keyframes: animation.effect?.getKeyframes?.() || [],
    timing: animation.effect?.getTiming?.() || {}
  })).filter(animation => animation.target);
  const animations = [
    ...liveAnimations,
    ...(window.__recreateLifecycleAnimations || [])
  ];
  const cssRules = [];
  const visitRules = rules => {
    for (const rule of Array.from(rules || [])) {
      cssRules.push(rule.cssText);
      if (rule.cssRules) visitRules(rule.cssRules);
    }
  };
  for (const sheet of Array.from(document.styleSheets)) {
    try { visitRules(sheet.cssRules); } catch {}
  }
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
  const assetData = {};
  await Promise.all(Array.from(assets)
    .filter(url => url.startsWith('blob:'))
    .map(async url => {
      try {
        const blob = await fetch(url).then(response => response.blob());
        assetData[url] = await new Promise((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => resolve(reader.result);
          reader.onerror = reject;
          reader.readAsDataURL(blob);
        });
      } catch {}
    }));
  return JSON.stringify({
    url: location.href,
    title: document.title,
    nodes,
    animations,
    css_rules: cssRules,
    asset_urls: Array.from(assets),
    asset_data: assetData
  });
})()
"#;
