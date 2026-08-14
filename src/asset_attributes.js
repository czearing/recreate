  const assetUrlAttributes = new Set([__URL_ATTRIBUTES__]);
  const assetCandidateAttributes = new Set([__CANDIDATE_ATTRIBUTES__]);
  const skippedAttributes = new Set([__SKIPPED_ATTRIBUTES__]);
  const assetSelector = '__ASSET_SELECTOR__';
  // What terminates a `url()` value is decided by what opened it. CSS Syntax hides two
  // productions behind one spelling: `url(` followed by a quote is a function token whose
  // argument is a string token, ending only at the matching unescaped quote, so a `)`
  // inside it is content; `url(` followed by anything else is a url token, ending at the
  // first unescaped whitespace or `)`, so a quote inside it is content. One character
  // class cannot express two terminators, and widening one class to admit the other's
  // terminator leaves it able to terminate neither.
  //
  // This matters because CSSOM never returns authored text. It serialises a URL as `url(`
  // plus *serialize a string*, which wraps the value in `"` and escapes exactly `"` and
  // `\`. A `)` or a `'` in the URL therefore arrives raw inside the quotes, as content.
  const unescapeCss = value =>
    value.replace(/\\(?:([0-9a-fA-F]{1,6})[ \t\n]?|([\s\S]))/g, (_, hex, char) =>
      hex ? String.fromCodePoint(parseInt(hex, 16)) : char);
  // Every `url()` value in a fragment of CSS text, unescaped. Escapes are resolved because
  // the emitted CSS this keys against is serialised by the same CSSOM, which resolves them
  // too; leaving them in would key the map on a spelling the text never contains.
  const cssUrls = function* (text) {
    let index = 0;
    while ((index = text.indexOf('url(', index)) >= 0) {
      if (index > 0 && /[-\w\\]/.test(text[index - 1])) { index += 4; continue; }
      let at = index + 4;
      while (/\s/.test(text[at])) at++;
      const quote = text[at] === '"' || text[at] === "'" ? text[at] : '';
      const ends = quote
        ? char => char === quote
        : char => char === ')' || /\s/.test(char);
      let value = '';
      for (at += quote ? 1 : 0; at < text.length && !ends(text[at]); at++) {
        if (text[at] === '\\') value += text[at++];
        value += text[at] ?? '';
      }
      // An unterminated value is a parse error. Resynchronising past it would consume the
      // rest of the sheet, so nothing after it is claimed.
      if (at >= text.length) return;
      let close = quote ? at + 1 : at;
      while (/\s/.test(text[close])) close++;
      index = close + 1;
      if (text[close] === ')') yield unescapeCss(value);
    }
  };
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
  // Every URL a walked element referenced. Resolving a reference and recording it are one
  // decision made in one place, so the set is reached by whatever traversal reached the
  // element. A query of its own would be a second traversal with its own scope, and the
  // two would answer differently for anything the walk found inside a shadow root.
  const assetUrls = new Set();
  const recordAssetUrl = url => {
    const resolved = resolveUrl(url);
    assetUrls.add(resolved);
    return resolved;
  };
  // The recorded attributes of one element. Values on an asset-bearing element are
  // resolved against the document base, so they are spelled the way the asset map is
  // keyed and the emitter's exact lookup can hit.
  //
  // An element that paints content no attribute addresses contributes one more, read from
  // the element itself. It is recorded here rather than beside each walk so that every
  // traversal which records attributes records the content too, and the two cannot drift.
  const recreateAttributes = (element, path) => {
    const localise = element.matches?.(assetSelector);
    return {
      ...Object.fromEntries(
        Array.from(element.attributes)
          .filter(attribute =>
            !attribute.name.startsWith('on') && !skippedAttributes.has(attribute.name))
          .map(attribute => [
            attribute.name,
            localise
              ? mapAttributeUrls(attribute.name, attribute.value, recordAssetUrl)
              : attribute.value
          ])
      ),
      ...recreateSurfaceAttributes(element, path)
    };
  };
  // The IDL attributes that do NOT reflect their content attribute, each paired with the
  // `default*` twin that does. For a form control the content attribute is the default,
  // never the state: `value` is `defaultValue`, `checked` is `defaultChecked`, `selected`
  // is `defaultSelected`. Typing, clicking or assigning the property updates the state and
  // never writes the attribute, so an attribute-derived record is structurally incapable of
  // carrying it — no amount of settling puts it there.
  //
  // Pairing each with its twin is what keeps this free of element shape tests. The engine
  // already knows each element's default, so "did this diverge?" is asked the same way for
  // every member, and a member whose default lives somewhere unusual — a `<textarea>`'s is
  // its child text, not an attribute — needs no case of its own.
  //
  // The one partition the spec forces is which state a control actually holds. A checkbox
  // or radio holds *checkedness*; its `value` is in the spec's "default/on" mode, where it
  // reflects the content attribute and falls back to the string "on". Reading `value` there
  // would report a divergence the page never made, because the fallback differs from the
  // empty default by construction. `file` is excluded for the opposite reason: its value is
  // a synthetic path that names a file the recreation cannot have. Every remaining type is
  // in "default" mode, where the property equals its own default and records nothing.
  const nonReflectingState = [
    { attribute: 'value', hosts: 'textarea,input:not([type=checkbox]):not([type=radio]):not([type=file])', live: e => e.value, base: e => e.defaultValue },
    { attribute: 'checked', hosts: 'input[type=checkbox],input[type=radio]', live: e => e.checked, base: e => e.defaultChecked },
    { attribute: 'selected', hosts: 'option', live: e => e.selected, base: e => e.defaultSelected }
  ];
  // What an element's live state says that its markup default does not. Keyed by the
  // content attribute it overrides, so a consumer asks one question of two sources; a
  // boolean is spelled the way the attribute would be, and `null` records a default that
  // was turned off, which no absent entry could express.
  const recreateControlState = element => {
    const state = {};
    for (const { attribute, hosts, live, base } of nonReflectingState) {
      if (!element.matches?.(hosts)) continue;
      const current = live(element);
      if (current === base(element)) continue;
      state[attribute] = typeof current === 'boolean' ? (current ? '' : null) : current;
    }
    return state;
  };
  // Every URL the recreation must contain bytes for: one per candidate rather than one
  // per element, plus any `url()` in a captured declaration or stylesheet rule.
  //
  // A generated box contributes its style map plus its `content`, which is a declaration the
  // generator emits from its own field. Enumerating only the map would leave the generator
  // localising a value this walk never saw, and the two would agree only for as long as the
  // baseline reduction happens to keep `content` in both places.
  const recreateAssetUrls = (nodes, cssRules) => {
    const assets = new Set(assetUrls);
    const boxes = node =>
      Object.values(node.pseudos || {}).map(b => ({ ...b.style, content: b.content }));
    for (const node of nodes) {
      for (const style of [node.style, ...boxes(node)]) {
        for (const value of Object.values(style || {})) {
          for (const url of cssUrls(String(value))) {
            assets.add(resolveUrl(url));
          }
        }
      }
    }
    for (const rule of cssRules) {
      for (const match of cssUrls(rule)) {
        const url = resolveUrl(match);
        if (!url.startsWith('data:')) assets.add(url);
      }
    }
    return assets;
  };
