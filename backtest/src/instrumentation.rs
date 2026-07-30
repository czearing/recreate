pub const PRE_DOCUMENT: &str = r#"
(() => {
  if (globalThis.__backtest) return;
  const state = {
    now: 0,
    next: 1,
    timers: [],
    rafs: [],
    errors: [],
    requests: [],
    layoutShifts: []
  };
  const recordLayoutShift = (entry) => {
    if (entry.hadRecentInput || !(entry.value > 0)) return;
    state.layoutShifts.push({
      value: Math.round(entry.value * 1000000) / 1000000,
      sources: Array.from(entry.sources || []).map((source) => {
        const node = source.node;
        if (!(node instanceof Element)) return "element";
        const label = node.getAttribute("aria-label") ||
          node.getAttribute("data-backtest-id") ||
          node.id ||
          node.textContent?.replace(/\s+/g, " ").trim().slice(0, 48);
        return label ? `${node.tagName.toLowerCase()} "${label}"` : node.tagName.toLowerCase();
      }).sort()
    });
  };
  let layoutShiftObserver = null;
  if (typeof PerformanceObserver !== "undefined" &&
      PerformanceObserver.supportedEntryTypes?.includes("layout-shift")) {
    layoutShiftObserver = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) recordLayoutShift(entry);
    });
    layoutShiftObserver.observe({ type: "layout-shift", buffered: true });
  }
  const nativeError = console.error.bind(console);
  console.error = (...args) => {
    state.errors.push(args.map((value) => String(value)).join(" "));
    nativeError(...args);
  };
  const nativeFetch = globalThis.fetch.bind(globalThis);
  globalThis.fetch = (input, init) => {
    const raw = typeof input === "string" ? input : input.url;
    const url = new URL(raw, location.href);
    const method = String(
      init?.method || (typeof Request !== "undefined" && input instanceof Request ? input.method : "GET")
    ).toUpperCase();
    const identity = url.origin === globalThis.location.origin
      ? `same-origin${url.pathname}${url.search}`
      : `${url.origin}${url.pathname}${url.search}`;
    state.requests.push(`${method} ${identity}`);
    return nativeFetch(input, init);
  };
  globalThis.setTimeout = (callback, delay = 0, ...args) => {
    const id = state.next++;
    state.timers.push({ id, due: state.now + Math.max(0, Number(delay) || 0), callback, args });
    return id;
  };
  globalThis.clearTimeout = (id) => {
    state.timers = state.timers.filter((timer) => timer.id !== id);
  };
  globalThis.setInterval = (callback, delay = 0, ...args) => {
    const id = state.next++;
    state.timers.push({
      id,
      due: state.now + Math.max(1, Number(delay) || 1),
      interval: Math.max(1, Number(delay) || 1),
      callback,
      args
    });
    return id;
  };
  globalThis.clearInterval = globalThis.clearTimeout;
  globalThis.requestAnimationFrame = (callback) => {
    const id = state.next++;
    state.rafs.push({ id, callback });
    return id;
  };
  globalThis.cancelAnimationFrame = (id) => {
    state.rafs = state.rafs.filter((frame) => frame.id !== id);
  };
  globalThis.requestIdleCallback = (callback) =>
    globalThis.setTimeout(() => callback({ didTimeout: false, timeRemaining: () => 50 }), 0);
  globalThis.cancelIdleCallback = globalThis.clearTimeout;
  globalThis.addEventListener("error", (event) => {
    state.errors.push(String(event.message || event.error || "error"));
  });
  globalThis.addEventListener("unhandledrejection", (event) => {
    state.errors.push(String(event.reason || "unhandled rejection"));
  });
  globalThis.__backtest = {
    advance(milliseconds) {
      const target = state.now + Math.max(0, Number(milliseconds) || 0);
      let guard = 0;
      while (guard < 10000) {
        guard++;
        state.timers.sort((a, b) => a.due - b.due || a.id - b.id);
        const timer = state.timers[0];
        if (!timer || timer.due > target) break;
        state.timers.shift();
        state.now = timer.due;
        if (typeof timer.callback === "function") timer.callback(...timer.args);
        else Function(String(timer.callback))();
        if (timer.interval) {
          timer.due += timer.interval;
          state.timers.push(timer);
        }
      }
      state.timers.sort((a, b) => a.due - b.due || a.id - b.id);
      if (guard >= 10000 && state.timers[0]?.due <= target) {
        throw new Error("virtual timer callback limit reached");
      }
      state.now = target;
      const frames = state.rafs.splice(0);
      for (const frame of frames) frame.callback(state.now);
      return { now: state.now, pendingTimers: state.timers.length, pendingFrames: state.rafs.length };
    },
    snapshot() {
      for (const entry of layoutShiftObserver?.takeRecords?.() || []) recordLayoutShift(entry);
      return {
        consoleErrors: [...state.errors],
        requests: [...state.requests],
        now: state.now,
        pendingTimers: state.timers.length,
        pendingFrames: state.rafs.length,
        layoutShifts: [...state.layoutShifts]
      };
    }
  };
})();
"#;
