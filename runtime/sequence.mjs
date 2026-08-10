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

// Replay begins at whichever value the captured DOM already holds, not at the first step, so
// the steps before that point were observed strictly BEFORE the capture. Wrapping past the
// end is what makes them reachable again, and reaching them does not extend the progression,
// it rewinds history. A progression the capture watched come back round has no beginning and
// must keep wrapping; one that never repeated stops on its last observed value, which is the
// same shape as `animation-iteration-count: 1` with `animation-fill-mode: forwards`.
//
// Absence of the fact is not evidence against it: data emitted before it was recorded says
// nothing either way, and treating silence as "does not repeat" would stop motion the tool
// used to reproduce correctly. Only a recorded `false` terminates.
function nextIndex(sequence, index) {
  const next = index + 1;
  if (next < sequence.steps.length) return next;
  return sequence.repeats === false ? -1 : 0;
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
  const arm = () => {
    timer = nextIndex(sequence, index) < 0
      ? null
      : clock.setTimeout(advance, sequence.steps[index].delay_ms);
  };
  const advance = () => {
    if (stopped) return;
    index = nextIndex(sequence, index);
    const step = sequence.steps[index];
    applySequenceValue(element, sequence, step.value);
    arm();
  };
  if (capturedIndex < 0) applySequenceValue(element, sequence, sequence.steps[index].value);
  arm();
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
