use crate::{asset_script, style_contract};

const CAPTURE: &str = r#"
(async () => {
  const props = [__STYLE_PROPERTIES__];
  const ignored = new Set(['SCRIPT','STYLE','NOSCRIPT','META','LINK','HEAD']), directionalBorders = [__DIRECTIONAL_BORDERS__];
  const styleMap = style => { const values = Object.fromEntries(props.map(p => [p, style.getPropertyValue(p)])); if (!values.border) for (const property of directionalBorders) values[property] = style.getPropertyValue(property); return values; };
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
  const animations = [
    ...liveAnimations,
    ...(window.__recreateLifecycleAnimations || [])
  ];
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
          if (states.length && !/:(hover|focus-visible|focus-within|focus|active)\b/.test(tail)) {
            continue;
          }
          if (!states.length && !reduced) continue;
          const base = selector.replace(dynamicState, '').trim();
          const pseudoElement = base.match(/::[\w-]+$/)?.[0] || '', query = base.slice(0, base.length - pseudoElement.length);
          if (!query) continue;
          try {
            for (const element of document.querySelectorAll(query)) {
              const captured = {
                target: pathOf(element),
                pseudo: states.length || pseudoElement ? `${states.join('')}${pseudoElement}` : null,
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
    animations,
    state_styles: stateStyles,
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
        .replace("__ASSET_CAPTURE__", asset_script::SOURCE)
}

#[cfg(test)]
#[path = "page_script_tests.rs"]
mod tests;
