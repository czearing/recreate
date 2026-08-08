// Recording the attribute and text changes that become sequences. Spliced into the
// lifecycle recorder, so it shares that scope's `pathOf` and its recorded output list.
    const trackedAttributes = new Set(['placeholder','title','aria-label','value']);
    const lastAttribute = new Map();
    const lastText = new Map();
    const recordedText = new Set();
    const textValue = element => (element.textContent || '').replace(/\s+/g, ' ').trim();    const trackableText = element => {
      if (!(element instanceof Element)) return '';
      const value = textValue(element);
      if (!value || value.length > 160 || element.childElementCount > 6) return '';
      if (element.querySelector('a,button,input,textarea,select,svg,img,video')) return '';
      return value;
    };
    const seedText = element => {
      if (!(element instanceof Element)) return;
      const value = trackableText(element);
      if (value) {
        lastText.set(pathOf(element), value);
      }
      for (const child of element.querySelectorAll('*')) {
        const childValue = trackableText(child);
        if (childValue) {
          lastText.set(pathOf(child), childValue);
        }
      }
    };
    for (const element of document.querySelectorAll('*')) {
      if (element.parentElement === document.documentElement) seedText(element);
    }
    const recordText = (element, now) => {
      const current = trackableText(element);
      if (!current) return false;
      const target = pathOf(element);
      const before = lastText.get(target);
      lastText.set(target, current);
      if (!before || before === current) return false;
      if (!recordedText.has(target)) {
        recordedText.add(target);
        window.__recreateAttributeMutations.push({
          target, attribute: 'textContent', value: before, time: 0
        });
      }
      window.__recreateAttributeMutations.push({
        target, attribute: 'textContent', value: current, time: now
      });
      return true;
    };
    new MutationObserver(mutations => {
      const now = performance.now() - start;
      for (const mutation of mutations) {
        if (mutation.type === 'characterData') {
          recordText(mutation.target.parentElement, now);
          continue;
        }
        if (mutation.type === 'childList') {
          if (recordText(mutation.target, now)) continue;
          for (const node of mutation.addedNodes) {
            if (!(node instanceof Element)) continue;
            recordText(node, now);
            for (const child of node.querySelectorAll('*')) recordText(child, now);
          }
          continue;
        }
        if (mutation.type !== 'attributes' || !trackedAttributes.has(mutation.attributeName)) continue;
        const element = mutation.target;
        const key = `${pathOf(element)}|${mutation.attributeName}`;
        const current = element.getAttribute(mutation.attributeName) || '';
        if (!lastAttribute.has(key) && mutation.oldValue) {
          window.__recreateAttributeMutations.push({
            target: pathOf(element), attribute: mutation.attributeName,
            value: mutation.oldValue, time: 0
          });
        }
        if (lastAttribute.get(key) !== current) {
          lastAttribute.set(key, current);
          window.__recreateAttributeMutations.push({
            target: pathOf(element), attribute: mutation.attributeName,
            value: current, time: now
          });
        }
      }
    }).observe(document.documentElement, {
      attributes: true, attributeOldValue: true, childList: true,
      characterData: true, characterDataOldValue: true, subtree: true
    });

