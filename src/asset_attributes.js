  const assetUrlAttributes = new Set([__URL_ATTRIBUTES__]);
  const assetCandidateAttributes = new Set([__CANDIDATE_ATTRIBUTES__]);
  const skippedAttributes = new Set([__SKIPPED_ATTRIBUTES__]);
  const assetSelector = '__ASSET_SELECTOR__';
  // A recorded rule as the artifact carries it: the text, with every reference resolved
  // against the sheet that held it and then written in its shortest unambiguous spelling.
  // The asset map is keyed by that same expression, so the emitter's substitution matches by
  // construction rather than by recognising a spelling. A relative path is not a spelling of
  // a URL — it is a URL plus a base — and the base is known here and nowhere later.
  const resolveCssRuleUrls = ({ text, base }) =>
    mapCssUrls(text, url => shortestUrlSpelling(resolveUrl(url, base)));
  // A reference to the document's own origin, written origin-relative. The capture rig's
  // ephemeral port is not a fact about the page: baking it in would make two runs of one
  // capture disagree, and would point any reference the capture could not download at a
  // server that stops existing when the run ends. Origin-relative is what the recreation
  // serves those bytes from anyway. A cross-origin reference keeps its absolute form, the
  // only spelling that names it.
  //
  // Nothing is lost by shortening: an origin-relative path is base-independent, so resolving
  // the result against the document base recovers the absolute URL exactly, which is how the
  // asset set is read back out of these strings.
  const shortestUrlSpelling = url => {
    try {
      const resolved = new URL(url);
      return resolved.origin === new URL(document.baseURI).origin
        ? resolved.href.slice(resolved.origin.length)
        : resolved.href;
    } catch { return url; }
  };
  // `document.baseURI`, not `location.href`. HTML resolves a relative reference against the
  // document base URL — the first `<base href>`, falling back to the document's own URL —
  // so the location is a near-synonym that is right only until a page carries a `<base>`,
  // and then wrong for every relative reference at once. Guarded by `base_tests`, which is
  // the only fixture here that gives the two operands different values.
  //
  // The base is a parameter because it is not always the document's: CSS 2.1 §4.3.4 makes a
  // stylesheet's own location the base for every reference inside it, and a sheet in a
  // subdirectory therefore answers differently from the page that linked it. The document
  // base stays the default, which is the right answer for every reference an element carries
  // and for an inline sheet, whose location is the document's. Guarded by `sheet_base_tests`.
  const resolveUrl = (url, base = document.baseURI) => {
    try { return new URL(url, base).href; } catch { return url; }
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
  // The rule texts arrive resolved and shortened, so this reads the very strings the
  // artifact will carry and resolves them back — origin-relative is base-independent, so
  // the absolute URL comes back exactly. Resolving against a different base here would let
  // the set and the text drift apart the moment one of the two stopped doing it.
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
    for (const text of cssRules) {
      for (const match of cssUrls(text)) {
        if (!match.startsWith('data:')) assets.add(resolveUrl(match));
      }
    }
    return assets;
  };
  // Both of the artifact's views of the page's stylesheets, from one expression. The rule
  // texts carry every reference resolved against the sheet that held it — the only stage
  // where that sheet's identity still exists — and the asset set is read back out of those
  // very strings, so what the recreation fetches and what it references cannot disagree.
  // Two sheets that authored the same rule contribute one line only when their references
  // genuinely resolve to the same bytes.
  const recreateCssAssets = (nodes, cssRules) => {
    const texts = [...new Set(cssRules.map(resolveCssRuleUrls))];
    return { texts, urls: recreateAssetUrls(nodes, texts) };
  };
