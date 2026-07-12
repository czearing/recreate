export const discoverIframeExpression = `(() => {
  const frames = Array.from(document.querySelectorAll('iframe')).map(frame => {
    const rect = frame.getBoundingClientRect();
    let accessible = false;
    try {
      accessible = Boolean(
        frame.contentDocument &&
        frame.contentWindow.location.origin === location.origin
      );
    } catch {}
    return {
      id: frame.id || null,
      title: frame.title || null,
      src: frame.src,
      sandbox: frame.getAttribute('sandbox'),
      accessible,
      clientLeft: frame.clientLeft,
      clientTop: frame.clientTop,
      rect: rect.toJSON()
    };
  }).filter(frame => frame.rect.width > 4 && frame.rect.height > 4);
  const boundary = frames.find(frame => frame.accessible);
  if (!boundary) return { frames };
  const frame = boundary.id
    ? document.getElementById(boundary.id)
    : Array.from(document.querySelectorAll('iframe'))
        .find(element => element.src === boundary.src);
  const button = frame.contentDocument.querySelector('button');
  if (!button) return { frames };
  let previousFocus = document.activeElement;
  while (previousFocus?.shadowRoot?.activeElement) {
    previousFocus = previousFocus.shadowRoot.activeElement;
  }
  window.__siteSpecFrame = frame;
  window.__siteSpecFrameButton = button;
  window.__siteSpecFramePreviousFocus = previousFocus;
  const buttonRect = button.getBoundingClientRect();
  return {
    frames,
    sameOrigin: boundary,
    button: {
      label: (
        button.getAttribute('aria-label') ||
        button.innerText ||
        button.textContent ||
        ''
      ).trim(),
      x: boundary.rect.x + boundary.clientLeft +
        buttonRect.x + buttonRect.width / 2,
      y: boundary.rect.y + boundary.clientTop +
        buttonRect.y + buttonRect.height / 2
    }
  };
})()`;

export const iframeStateExpression = `(() => {
  const frame = window.__siteSpecFrame;
  const documentRoot = frame?.contentDocument;
  if (!frame) return null;
  if (!documentRoot) {
    return {
      unavailable: true,
      url: null,
      declaredSrc: frame.src,
      title: frame.title,
      text: '',
      parentStatus: document.querySelector('#status')?.innerText || null,
      nodes: [],
      focus: null
    };
  }
  const frameRect = frame.getBoundingClientRect();
  const pathFor = element => {
    const parts = [];
    let current = element;
    while (current && current.nodeType === Node.ELEMENT_NODE) {
      const siblings = Array.from(current.parentElement?.children || [])
        .filter(sibling => sibling.tagName === current.tagName);
      parts.unshift(
        current.tagName.toLowerCase() + ':nth-of-type(' +
        (Math.max(0, siblings.indexOf(current)) + 1) + ')'
      );
      current = current.parentElement;
    }
    return 'frame(' + (frame.title || frame.id || 'same-origin') + ')>' +
      parts.join('>');
  };
  const nodes = Array.from(documentRoot.querySelectorAll('*')).flatMap(element => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    if (
      rect.width <= 0 ||
      rect.height <= 0 ||
      style.display === 'none' ||
      style.visibility === 'hidden'
    ) return [];
    return [{
      path: pathFor(element),
      tag: element.tagName.toLowerCase(),
      attrs: {
        id: element.id || undefined,
        class: element.getAttribute('class') || undefined,
        role: element.getAttribute('role') || undefined,
        ariaExpanded: element.getAttribute('aria-expanded') || undefined,
        ariaSelected: element.getAttribute('aria-selected') || undefined,
        dataState: element.getAttribute('data-state') || undefined
      },
      text: (element.innerText || element.getAttribute('aria-label') || '')
        .replace(/\\s+/g, ' ').trim().slice(0, 300),
      rect: {
        x: frameRect.x + frame.clientLeft + rect.x,
        y: frameRect.y + frame.clientTop + rect.y,
        width: rect.width,
        height: rect.height
      },
      style: {
        display: style.display,
        position: style.position,
        padding: style.padding,
        color: style.color,
        backgroundColor: style.backgroundColor,
        border: style.border,
        borderRadius: style.borderRadius,
        fontFamily: style.fontFamily,
        fontSize: style.fontSize,
        lineHeight: style.lineHeight
      }
    }];
  });
  return {
    url: frame.contentWindow.location.href,
    title: documentRoot.title,
    text: documentRoot.body.innerText.replace(/\\s+/g, ' ').trim(),
    parentStatus: document.querySelector('#status')?.innerText || null,
    nodes,
    focus: documentRoot.activeElement && documentRoot.activeElement !== documentRoot.body
      ? {
          tag: documentRoot.activeElement.tagName.toLowerCase(),
          id: documentRoot.activeElement.id || null
        }
      : null
  };
})()`;

export const cleanupIframeExpression = `(() => {
  const previous = window.__siteSpecFramePreviousFocus;
  if (previous && previous !== document.body) previous.focus();
  else document.activeElement?.blur?.();
  delete window.__siteSpecFrame;
  delete window.__siteSpecFrameButton;
  delete window.__siteSpecFramePreviousFocus;
})()`;
