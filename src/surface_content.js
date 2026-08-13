  const surfaceAttribute = '__SURFACE_ATTRIBUTE__';
  const surfaceScheme = '__SURFACE_SCHEME__';
  const surfaceAssets = {};
  const surfaceBlockers = [];
  // What a surface of these dimensions exports when nothing has been drawn on it. The
  // comparison needs no threshold and no pixel walk: an untouched surface and a surface
  // whose buffer was discarded export the same bytes as one made here and never used.
  const blankSurfaces = new Map();
  const blankSurface = element => {
    const key = `${element.width}x${element.height}`;
    if (!blankSurfaces.has(key)) {
      const blank = document.createElement('canvas');
      blank.width = element.width;
      blank.height = element.height;
      blankSurfaces.set(key, blank.toDataURL());
    }
    return blankSurfaces.get(key);
  };
  // Why the capture could not obtain what an element painted. Both causes fail the same
  // premise — the user saw pixels this read did not return — so they are recorded in one
  // voice and differ only in the reason they name.
  const surfaceUnreadable = (path, reason) => {
    surfaceBlockers.push(
      `the content painted on ${path} could not be read (${reason}); ` +
      'the element is emitted at its measured size with nothing in it');
    return {};
  };
  // Whether a blank export means the surface was unreadable rather than empty. Canvas
  // contexts are exclusive, so a surface already bound to a non-2D context answers null
  // here, and a non-preserving drawing buffer is exactly such a surface read after its
  // frame was presented. Asking creates a context on a surface that had none, so this runs
  // only on surfaces already about to be recorded as empty, and never on one that exported.
  const discardedDrawingBuffer = element =>
    typeof element.getContext === 'function' && element.getContext('2d') === null;
  // The one attribute a drawing surface does not have. Read once, here, because the buffer
  // exists only while the page is live.
  const recreateSurfaceAttributes = (element, path) => {
    if (typeof element.toDataURL !== 'function') return {};
    let painted;
    try {
      painted = element.toDataURL();
    } catch (error) {
      return surfaceUnreadable(path, error.name);
    }
    if (!painted || painted === blankSurface(element)) {
      if (!discardedDrawingBuffer(element)) return {};
      return surfaceUnreadable(path, 'its drawing buffer was discarded after the frame was presented');
    }
    const key = surfaceScheme + path;
    surfaceAssets[key] = painted;
    return { [surfaceAttribute]: key };
  };
  const recreateSurfaceAssets = () => surfaceAssets;
  const recreateSurfaceBlockers = () => surfaceBlockers;


