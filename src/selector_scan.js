/* The one reader of selector and value text in the capture.

   A selector is grammar interleaved with data: a quoted attribute value holds arbitrary
   text, and a functional pseudo-class holds a whole selector list. Every question the
   capture asks about a selector — where does this list end, which construct owns this
   comma, where does this function close — is a question about the grammar, so all of them
   have to skip the data and count the nesting. Balanced delimiters are not a regular
   language, so no pattern without a depth counter answers any of them, and a stage that
   answers one with `split(',')` does not fail loudly: it produces a fragment that is still
   a selector, matching something the author never wrote, or throwing where the throw is
   caught and the rule leaves no trace.

   Stated once so the state capture, the activation probe and the generated-box scan read a
   selector the same way. A selector read two ways is a selector one of them reads wrongly. */

/* Every character a scan is entitled to act on, with the paren depth it sits at. Escapes and
   quoted strings are consumed whole, so nothing inside either is ever offered. */
const scanValue = (text, from, accept) => {
  let depth = 0;
  for (let index = from; index < text.length; index++) {
    const char = text[index];
    if (char === '\\') { index++; continue; }
    if (char === '"' || char === "'") {
      for (index++; index < text.length && text[index] !== char; index++) {
        if (text[index] === '\\') index++;
      }
      continue;
    }
    if (char === '(') depth++;
    else if (char === ')') depth--;
    if (accept(char, depth, index)) return index;
  }
  return -1;
};
const closingParen = (text, open) => scanValue(text, open, (char, depth) => char === ')' && !depth);
const topLevelComma = text => scanValue(text, 0, (char, depth) => char === ',' && !depth);

/* The members of a selector list. A list is separated by top-level commas only; the commas
   inside `:is(.a, .b)`, `:not([a],[b])` and `[title="a,b"]` belong to their own construct. */
const selectorMembers = text => {
  const members = [];
  let start = 0;
  for (;;) {
    const cut = scanValue(text, start, (char, depth) => char === ',' && !depth);
    if (cut < 0) break;
    members.push(text.slice(start, cut).trim());
    start = cut + 1;
  }
  members.push(text.slice(start).trim());
  return members.filter(Boolean);
};

/* The compound a member ends on — its subject. A combinator only separates compounds at the
   top level, so the whitespace in `:has(> .a)` and in `[title="a b"]` divides nothing. */
const lastCompound = member => {
  let start = 0;
  for (;;) {
    const cut = scanValue(member, start, (char, depth) => !depth && ' >+~\t\n'.includes(char));
    if (cut < 0) return member.slice(start).trim();
    start = cut + 1;
  }
};

