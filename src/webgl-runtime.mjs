export const rafControlSource = `(() => {
  if (window.__siteSpecRafControl) return;
  const nativeRaf = window.requestAnimationFrame.bind(window);
  const nativeNow = performance.now.bind(performance);
  let paused = false;
  let queued = [];
  let currentTime = nativeNow();
  Object.defineProperty(performance, 'now', {
    configurable: true,
    value: () => currentTime
  });
  window.requestAnimationFrame = callback => nativeRaf(timestamp => {
    if (paused) queued.push(callback);
    else {
      currentTime = timestamp;
      callback(timestamp);
    }
  });
  window.__siteSpecRafControl = {
    pause() {
      paused = true;
    },
    step(deltaMs = 0) {
      currentTime += deltaMs;
      const callbacks = queued;
      queued = [];
      callbacks.forEach(callback => callback(currentTime));
      return callbacks.length;
    },
    resume() {
      paused = false;
      const callbacks = queued;
      queued = [];
      callbacks.forEach(callback => nativeRaf(callback));
    },
    status() {
      return { paused, queued: queued.length, currentTime };
    }
  };
})()`;

export const discoverWebglCanvasExpression = `(() => {
  const canvas = Array.from(document.querySelectorAll('canvas'))
    .filter(element => {
      const rect = element.getBoundingClientRect();
      const gl = element.getContext('webgl2') || element.getContext('webgl');
      return gl && rect.width > 100 && rect.height > 100;
    })
    .sort((left, right) => {
      const a = left.getBoundingClientRect();
      const b = right.getBoundingClientRect();
      return b.width * b.height - a.width * a.height;
    })[0];
  if (!canvas || !window.__siteSpecRafControl) return null;
  window.__siteSpecWebglCanvas = canvas;
  const rect = canvas.getBoundingClientRect();
  return {
    label: canvas.getAttribute('aria-label') || 'WebGL canvas',
    rect: rect.toJSON()
  };
})()`;

export const cleanupWebglExpression = `window.__siteSpecRafControl?.resume();
  delete window.__siteSpecWebglCanvas`;

export const webglStepSignatureExpression = (deltaMs) => `(() => {
  const callbacks = window.__siteSpecRafControl.step(${Number(deltaMs)});
  const canvas = window.__siteSpecWebglCanvas;
  const gl = canvas?.getContext('webgl2') || canvas?.getContext('webgl');
  if (!gl) return null;
  gl.finish();
  const width = gl.drawingBufferWidth;
  const height = gl.drawingBufferHeight;
  const pixels = new Uint8Array(width * height * 4);
  gl.readPixels(0, 0, width, height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
  const size = 32;
  const sampledGrid = [];
  let hash = 2166136261;
  let red = 0, green = 0, blue = 0, alpha = 0;
  for (let offset = 0; offset < pixels.length; offset += 4) {
    for (let channel = 0; channel < 4; channel++) {
      hash ^= pixels[offset + channel];
      hash = Math.imul(hash, 16777619);
    }
    red += pixels[offset];
    green += pixels[offset + 1];
    blue += pixels[offset + 2];
    alpha += pixels[offset + 3];
  }
  for (let gy = 0; gy < size; gy++) {
    for (let gx = 0; gx < size; gx++) {
      const x = Math.min(width - 1, Math.floor((gx + 0.5) * width / size));
      const y = Math.min(height - 1, Math.floor((gy + 0.5) * height / size));
      const offset = (y * width + x) * 4;
      sampledGrid.push([
        pixels[offset],
        pixels[offset + 1],
        pixels[offset + 2],
        pixels[offset + 3]
      ]);
    }
  }
  const count = width * height;
  return {
    callbacks,
    width,
    height,
    sampledGrid,
    sampleHash: (hash >>> 0).toString(16).padStart(8, '0'),
    meanRgba: [red / count, green / count, blue / count, alpha / count]
  };
})()`;
