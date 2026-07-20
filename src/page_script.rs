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
    if (ignored.has(element.tagName) || element.hasAttribute('data-recreate-startup')) return;
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
