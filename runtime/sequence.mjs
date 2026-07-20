export function applySequenceValue(element, sequence, value) {
  if (sequence.attribute === 'textContent') element.textContent = value;
  else element.setAttribute(sequence.attribute, value);
}

function currentValue(element, sequence) {
  return sequence.attribute === 'textContent'
    ? element.textContent
    : element.getAttribute?.(sequence.attribute);
}

export function startSequence(element, sequence, clock = globalThis) {
  if (!element || sequence.steps.length < 2) return () => {};
  const captured = currentValue(element, sequence);
  let index = sequence.steps.findIndex(step => step.value === captured);
  if (index < 0) index = 0;
  let timer = null;
  let stopped = false;
  const advance = () => {
    if (stopped) return;
    const step = sequence.steps[index];
    applySequenceValue(element, sequence, step.value);
    index = (index + 1) % sequence.steps.length;
    timer = clock.setTimeout(advance, step.delay_ms);
  };
  advance();
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
