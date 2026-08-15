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
  // Every `url()` value in a fragment of CSS text, with the span it occupies, unescaped.
  // Escapes are resolved because the emitted CSS this keys against is serialised by the
  // same CSSOM, which resolves them too; leaving them in would key the map on a spelling
  // the text never contains.
  const cssUrlTokens = function* (text) {
    let index = 0;
    while ((index = text.indexOf('url(', index)) >= 0) {
      const start = index;
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
      if (text[close] === ')') yield { value: unescapeCss(value), start, end: index };
    }
  };
  // Every `url()` value in a fragment of CSS text.
  const cssUrls = function* (text) {
    for (const { value } of cssUrlTokens(text)) yield value;
  };
  // The same text with every `url()` value passed through `map`, and nothing between them
  // disturbed. The replacement is always written quoted and with `"` and `\` escaped, which
  // is what CSSOM's own *serialize a string* produces: a `)` or a `'` may appear raw inside
  // a URL, and writing one back unquoted would close the token early and strand its tail.
  const mapCssUrls = (text, map) => {
    let out = '';
    let at = 0;
    for (const { value, start, end } of cssUrlTokens(text)) {
      out += text.slice(at, start) + `url("${map(value).replace(/[\\"]/g, '\\$&')}")`;
      at = end;
    }
    return out + text.slice(at);
  };
