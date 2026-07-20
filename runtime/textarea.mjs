function setter(prototype) {
  return Object.getOwnPropertyDescriptor(prototype, 'value')?.set;
}

export function setControlValue(element, value, environment = globalThis) {
  const textArea = environment.HTMLTextAreaElement;
  const input = environment.HTMLInputElement;
  const prototype = textArea && element instanceof textArea
    ? textArea.prototype
    : input && element instanceof input
      ? input.prototype
      : Object.getPrototypeOf(element);
  const write = setter(prototype);
  if (!write) throw new TypeError('control value setter is unavailable');
  write.call(element, value);
}

export function replayControlValue(element, value, environment = globalThis) {
  setControlValue(element, value, environment);
  const EventType = environment.Event;
  element.dispatchEvent(new EventType('input', { bubbles: true }));
  element.dispatchEvent(new EventType('change', { bubbles: true }));
}

export function submitIntent(event) {
  if (event.isComposing || event.key !== 'Enter') return 'none';
  return event.shiftKey ? 'newline' : 'submit';
}
