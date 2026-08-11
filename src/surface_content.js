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
  // The one attribute a drawing surface does not have. Read once, here, because the buffer
  // exists only while the page is live.
  const recreateSurfaceAttributes = (element, path) => {
    if (typeof element.toDataURL !== 'function') return {};
    let painted;
    try {
      painted = element.toDataURL();
    } catch (error) {
      surfaceBlockers.push(
        `the content painted on ${path} could not be read (${error.name}); ` +
        'the element is emitted at its measured size with nothing in it');
      return {};
    }
    if (!painted || painted === blankSurface(element)) return {};
    const key = surfaceScheme + path;
    surfaceAssets[key] = painted;
    return { [surfaceAttribute]: key };
  };
  const recreateSurfaceAssets = () => surfaceAssets;
  const recreateSurfaceBlockers = () => surfaceBlockers;


