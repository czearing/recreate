use serde::Deserialize;

const CANDIDATES_TEMPLATE: &str = r#"
(() => {
__NODE_PATH__
  const visible = element => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' &&
      style.visibility !== 'hidden';
  };
  const labelOf = element => (
    element.getAttribute('aria-label') ||
    element.getAttribute('placeholder') ||
    element.innerText ||
    element.value ||
    ''
  ).replace(/\s+/g, ' ').trim().slice(0, 120);
  const controls = Array.from(document.querySelectorAll(
  'a[href],button,[role="button"],[role="tab"],[aria-haspopup],[aria-expanded],' +
  '[aria-pressed],[aria-selected],summary,' +
  'input:not([type="hidden"]),textarea,select,[tabindex]:not([tabindex="-1"])'
  ));
  const modal = Array.from(document.querySelectorAll(
    '[aria-modal="true"],[role="dialog"]'
  )).filter(visible).at(-1);
  return controls.filter(element =>
    visible(element) &&
    (!modal || modal.contains(element)) &&
    !element.matches('[aria-hidden="true"],[data-tabster-dummy],[role="none"],[role="presentation"]') &&
    !(element.getAttribute('role') === 'button' &&
      element.querySelector('button,a[href],[role="button"]')) &&
    !(element.type === 'submit' && element.closest('form')) &&
    !element.closest('[contenteditable="true"]')
  ).map(element => {
   const label = labelOf(element);
    return {
    path: pathOf(element),
    tag: element.tagName.toLowerCase(),
    label,
    occurrence: controls.filter(candidate =>
      candidate.tagName === element.tagName && labelOf(candidate) === label
    ).indexOf(element),
    disabled: !!element.disabled || element.getAttribute('aria-disabled') === 'true',
    navigates: element.matches('a[href]'),
    state_control: element.getAttribute('role') === 'tab' ||
      element.hasAttribute('aria-pressed') || element.hasAttribute('aria-selected')
  }});
})()
"#;

pub(super) const PREFLIGHT: &str = r#"
(() => {
  let hash = 2166136261;
  const add = value => {
    const text = String(value);
    for (let index = 0; index < text.length; index++) {
      hash ^= text.charCodeAt(index);
      hash = Math.imul(hash, 16777619);
    }
  };
  add(`${document.documentElement.scrollWidth}:${document.documentElement.scrollHeight}`);
  for (const element of document.querySelectorAll('*')) {
    const rect = element.getBoundingClientRect();
    add(element.tagName);
    add(element.childElementCount);
    add(`${Math.round(rect.x * 2)}:${Math.round(rect.y * 2)}:${Math.round(rect.width * 2)}:${Math.round(rect.height * 2)}`);
    add(`${element.scrollLeft}:${element.scrollTop}`);
    const style = getComputedStyle(element);
    for (const name of ['color','background-color','border-color','box-shadow',
      'outline-color','outline-style','outline-width','opacity','transform']) {
      add(style.getPropertyValue(name));
    }
    for (const name of ['aria-expanded', 'aria-pressed', 'aria-selected', 'disabled', 'hidden']) {
      add(element.getAttribute(name) || '');
    }
  }
  return `${document.querySelectorAll('*').length}:${hash >>> 0}`;
})()
"#;

/// Waits for the page to stop reacting to an action, giving up after `deadline_ms`.
///
/// The deadline is a parameter rather than a substring rewritten by the caller: the script
/// carries other durations, and a caller patching one number by text cannot tell them apart.
pub(super) fn settle(deadline_ms: u64) -> String {
    format!(
        r#"
new Promise(resolve => {{
  const started = performance.now();
  const networkQuiet = {network};
  let cleanFrames = 0;
  const observer = new MutationObserver(() => {{ cleanFrames = 0; }});
  observer.observe(document, {{
    attributes: true, childList: true, characterData: true, subtree: true
  }});
  const sample = () => {{
    const running = document.getAnimations({{ subtree: true }})
      .some(animation => animation.playState === 'running');
    cleanFrames = running || !networkQuiet() ? 0 : cleanFrames + 1;
    if (cleanFrames >= 2 || performance.now() - started >= {deadline_ms}) {{
      observer.disconnect();
      resolve(cleanFrames >= 2);
    }} else {{
      requestAnimationFrame(sample);
    }}
  }};
  requestAnimationFrame(sample);
}})
"#,
        network = crate::network_quiet::js_gate(),
    )
}

const ACTION_SCOPE_TEMPLATE: &str = r#"
trigger => {
  globalThis.__recreateCaptureScope?.observer?.disconnect();
__NODE_PATH__
  const paths = new Set(trigger ? [trigger] : []);
  // Reaching an offscreen control means scrolling to it, and that scroll is the harness
  // moving rather than the page reacting. It cannot simply be discarded when it happens,
  // because a scroll event is delivered asynchronously and arrives well after the approach
  // has returned. So the approach reports where it left each scroller, and a scroll still
  // sitting at exactly that position is recognised as its own rather than the page's.
  const approached = new Map();
  const at = element => `${element.scrollLeft}:${element.scrollTop}`;
  const settle = elements => elements.forEach(element => approached.set(element, at(element)));
  const add = node => {
    if (node?.nodeType === Node.TEXT_NODE) node = node.parentElement;
    if (!(node instanceof Element)) return;
    const path = pathOf(node);
    if (path) paths.add(path);
    node.querySelectorAll('*').forEach(child => {
      const childPath = pathOf(child);
      if (childPath) paths.add(childPath);
    });
  };
  const observer = new MutationObserver(records => records.forEach(record => {
    add(record.target);
    record.addedNodes.forEach(add);
  }));
  observer.observe(document, {
    attributes: true, childList: true, characterData: true, subtree: true
  });
  const focus = event => add(event.target);
  const scroll = event => {
    const element =
      event.target === document ? document.scrollingElement : event.target;
    if (approached.get(element) === at(element)) return;
    add(element);
  };
  addEventListener('focusin', focus, true);
  addEventListener('scroll', scroll, true);
  globalThis.__recreateCaptureScope = {paths, observer, focus, scroll, settle};
}
"#;

pub(super) const TAKE_SCOPE: &str = r#"
(() => {
  const scope = globalThis.__recreateCaptureScope;
  scope?.observer.disconnect();
  removeEventListener('focusin', scope?.focus, true);
  removeEventListener('scroll', scope?.scroll, true);
  delete globalThis.__recreateCaptureScope;
  return [...(scope?.paths || [])];
})()
"#;

#[derive(Clone, Deserialize)]
pub(super) struct Candidate {
    pub(super) path: String,
    pub(super) tag: String,
    pub(super) label: String,
    pub(super) occurrence: usize,
    pub(super) disabled: bool,
    pub(super) navigates: bool,
    pub(super) state_control: bool,
}

impl Candidate {
    pub(super) fn uses_text_entry(&self) -> bool {
        crate::interactions_input::text_entry(&self.tag)
    }
}

/// The discovery script, with the shared path definition spliced in. Every consumer asks for
/// it here rather than for the template, so no caller can evaluate a script whose paths were
/// never defined.
pub fn candidates() -> String {
    crate::node_path::embed(CANDIDATES_TEMPLATE)
}

/// The scope recorder, spliced from the same owner for the same reason.
pub fn action_scope() -> String {
    crate::node_path::embed(ACTION_SCOPE_TEMPLATE)
}
