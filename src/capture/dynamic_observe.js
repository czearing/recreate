(async () => {
__LIFECYCLE_SETTLE__
  // Everything here is measured on the recorder's clock rather than on this loop's own,
  // because the loop attaches partway through the page's life. Measuring only what it
  // witnesses itself asserts that a page mid-progression has no history, and that a
  // progression which finished long before the loop started has only just changed.
  const origin = window.__recreateLifecycleStart || 0;
  const elapsed = () => performance.now() - origin;
  // What the page has written that is still news, when the last of it arrived, and the
  // widest gap the page has already recovered from.
  //
  // A group that has proven its cycle is described, so its further changes are motion the
  // recorder has already recorded rather than information it lacks. A group that has not
  // proven one is an absence of proof and not a promise — three values may be all there ever
  // were — so it holds the recorder only while the page's own cadence says another could
  // still arrive.
  const reading = () => {
    const events = window.__recreateAttributeMutations || [];
    const groups = new Map();
    // Seeded empty on purpose. The interval from the recorder's origin to the page's first
    // change is silence the page never recovered from, so counting it as a gap asserts a
    // cadence from the one interval that demonstrates none — and on a page whose first
    // attribute change lands seconds in, that invented gap is what the recorder then waits
    // out, every time, to its ceiling.
    const times = [];
    for (const event of events) {
      const key = `${event.target}|${event.attribute}`;
      const group = groups.get(key) || { values: [], times: [] };
      if (group.values.at(-1) !== event.value) {
        group.values.push(event.value);
        group.times.push(Number(event.time) || 0);
      }
      groups.set(key, group);
      times.push(Number(event.time) || 0);
    }
    let news = 0;
    let lastNews = 0;
    for (const { values, times: written } of groups.values()) {
      let cycle = values.length;
      for (let size = 1; size <= Math.floor(values.length / 2); size++) {
        if (values.every((value, index) => value === values[index % size])) {
          cycle = size;
          break;
        }
      }
      if (values.length < 3 || cycle === values.length) {
        news += values.length;
        lastNews = Math.max(lastNews, written.at(-1) || 0);
      }
    }
    const longestGap = lifecycleLongestGap(times);

    return { news, lastNews, longestGap };
  };
  const frame = () => new Promise(resolve => requestAnimationFrame(() => resolve()));
  const started = elapsed();
  for (;;) {
    await frame();
    const now = elapsed();
    const { news, lastNews, longestGap } = reading();
    // Nothing outstanding means nothing to wait for. Something outstanding means outwaiting the
    // widest gap the page has shown by the shortest gap the gate would even accept, because a
    // window of exactly the widest gap is a race against the page's own next step, and a page
    // that has changed once has shown no cadence at all yet.
    const sinceChange = news ? now - lastNews : now;
    const quiet = news ? longestGap + __STABLE_GAP_MS__ : longestGap;
    if (lifecycleSettled(now - started, sinceChange, false, quiet)) {
      return now - started;
    }
  }
})()
