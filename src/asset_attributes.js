  const assetUrlAttributes = new Set([__URL_ATTRIBUTES__]);
  const assetCandidateAttributes = new Set([__CANDIDATE_ATTRIBUTES__]);
  const skippedAttributes = new Set([__SKIPPED_ATTRIBUTES__]);
  const assetSelector = '__ASSET_SELECTOR__';
  const cssUrlPattern = /url\(["']?([^"')]+)["']?\)/g;
  const resolveUrl = url => {
    try { return new URL(url, location.href).href; } catch { return url; }
  };
  // Maps every URL in an attribute value through `map`, preserving descriptors and the
  // authored spacing. Two positions, as the srcset grammar defines them: a URL runs to
  // the next whitespace, so a comma inside it is data and only trailing commas end the
  // candidate; a descriptor run ends at the next comma.
  const mapAttributeUrls = (name, value, map) => {
    if (!value) return value;
    if (assetUrlAttributes.has(name)) return map(value);
    if (!assetCandidateAttributes.has(name)) return value;
    const indexOf = (text, pattern) => {
      const found = text.search(pattern);
      return found < 0 ? text.length : found;
    };
    let output = '', rest = value;
    while (rest) {
      const start = rest.search(/[^\s,]/);
      if (start < 0) { output += rest; break; }
      output += rest.slice(0, start);
      rest = rest.slice(start);
      const token = rest.slice(0, indexOf(rest, /\s/));
      const url = token.replace(/,+$/, '');
      output += map(url) + token.slice(url.length);
      rest = rest.slice(token.length);
      if (url.length < token.length) continue;
      const stop = indexOf(rest, /,/);
      output += rest.slice(0, stop);
      rest = rest.slice(stop);
    }
    return output;
  };
  // The recorded attributes of one element. Values on an asset-bearing element are
  // resolved against the document base, so they are spelled the way the asset map is
  // keyed and the emitter's exact lookup can hit.
  const recreateAttributes = element => {
    const localise = element.matches?.(assetSelector);
    return Object.fromEntries(
      Array.from(element.attributes)
        .filter(attribute =>
          !attribute.name.startsWith('on') && !skippedAttributes.has(attribute.name))
        .map(attribute => [
          attribute.name,
          localise
            ? mapAttributeUrls(attribute.name, attribute.value, resolveUrl)
            : attribute.value
        ])
    );
  };
  // Every URL the recreation must contain bytes for: one per candidate rather than one
  // per element, plus any `url()` in a captured declaration or stylesheet rule.
  const recreateAssetUrls = (nodes, cssRules) => {
    const assets = new Set();
    for (const element of document.querySelectorAll(assetSelector)) {
      for (const attribute of element.attributes) {
        mapAttributeUrls(attribute.name, attribute.value, url => {
          assets.add(resolveUrl(url));
          return url;
        });
      }
    }
    for (const node of nodes) {
      for (const style of [node.style, node.before?.style, node.after?.style]) {
        for (const value of Object.values(style || {})) {
          for (const match of String(value).matchAll(cssUrlPattern)) {
            assets.add(resolveUrl(match[1]));
          }
        }
      }
    }
    for (const rule of cssRules) {
      for (const match of rule.matchAll(cssUrlPattern)) {
        const url = resolveUrl(match[1]);
        if (!url.startsWith('data:')) assets.add(url);
      }
    }
    return assets;
  };
