const textLayouts = new WeakMap();

function applyTextValue(element, value) {
  const textNodes = [...(element.childNodes || [])]
    .filter(node => node.nodeType === 3);
  if (!textNodes.length) {
    element.textContent = value;
    return;
  }
  let layout = textLayouts.get(element);
  if (!layout) {
    layout = textNodes.map(node => node.nodeValue || '');
    textLayouts.set(element, layout);
  }
  let offset = 0;
  for (let index = 0; index < textNodes.length - 1; index++) {
    const remainingNodes = textNodes.length - index - 1;
    const available = value.length - offset;
    const captured = layout[index] || '';
    const length = value.startsWith(captured, offset)
      ? captured.length
      : Math.max(1, Math.min(captured.length || 1, available - remainingNodes));
    textNodes[index].nodeValue = value.slice(offset, offset + length);
    offset += length;
  }
  textNodes.at(-1).nodeValue = value.slice(offset);
}

export function applySequenceValue(element, sequence, value) {
  if (sequence.attribute !== 'textContent') {
    element.setAttribute(sequence.attribute, value);
    return;
  }
  applyTextValue(element, value);
}

const normalizeText = value => (value || '').replace(/\s+/g, ' ').trim();

function currentValue(element, sequence) {
  return sequence.attribute === 'textContent'
    ? normalizeText(element.textContent)
    : element.getAttribute?.(sequence.attribute);
}

export function startSequence(element, sequence, clock = globalThis) {
  if (!element || sequence.steps.length < 2 || clock.__recreateFreezeSequences) {
    return () => {};
  }
  const captured = currentValue(element, sequence);
  let index = sequence.steps.findIndex(step =>
    normalizeText(step.value) === captured
  );
  const capturedIndex = index;
  if (index < 0) index = 0;
  let timer = null;
  let stopped = false;
  const advance = () => {
    if (stopped) return;
    index = (index + 1) % sequence.steps.length;
    const step = sequence.steps[index];
    applySequenceValue(element, sequence, step.value);
    timer = clock.setTimeout(advance, step.delay_ms);
  };
  if (capturedIndex < 0) applySequenceValue(element, sequence, sequence.steps[index].value);
  timer = clock.setTimeout(advance, sequence.steps[index].delay_ms);
  return () => {
    stopped = true;
    if (timer !== null) clock.clearTimeout(timer);
  };
}

export function startSequences(root, sequences, clock = globalThis) {
  const stops = [];
  for (const element of root.querySelectorAll('[data-recreate-sequence]')) {
    for (const raw of element.dataset.recreateSequence.split(',')) {
      const sequence = sequences[Number(raw)];
      if (sequence) stops.push(startSequence(element, sequence, clock));
    }
  }
  return () => stops.forEach(stop => stop());
}
