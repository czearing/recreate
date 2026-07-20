export function fakeClock() {
  let now = 0;
  let nextId = 1;
  const timers = new Map();
  const setTimeout = (callback, delay) => {
    const id = nextId++;
    timers.set(id, { at: now + delay, callback });
    return id;
  };
  const clearTimeout = id => timers.delete(id);
  const tick = milliseconds => {
    const end = now + milliseconds;
    while (true) {
      const due = [...timers.entries()]
        .filter(([, timer]) => timer.at <= end)
        .sort((left, right) => left[1].at - right[1].at)[0];
      if (!due) break;
      timers.delete(due[0]);
      now = due[1].at;
      due[1].callback();
    }
    now = end;
  };
  return { setTimeout, clearTimeout, tick, pending: () => timers.size };
}

export class FakeElement {
  constructor() {
    this.attributes = new Map();
    this.dataset = {};
    this.events = [];
    this.textContent = '';
    this._value = '';
  }
  setAttribute(name, value) {
    this.attributes.set(name, value);
  }
  getAttribute(name) {
    return this.attributes.get(name) ?? null;
  }
  dispatchEvent(event) {
    this.events.push(event.type);
    return true;
  }
}
