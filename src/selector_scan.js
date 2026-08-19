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

const PSEUDO_NAME = /^::?[\w-]+/;
/* Every state a page enters and leaves. A selector is reduced against all of them, because a
   probe or a subject has to match the page as it rests; which of them a record can replay is
   a narrower question, asked by the stage that records. */
const STATE_NAME = /^:(hover|focus-visible|focus-within|focus|active|visited|target)\b/;
const RELATIONAL = /^:has\(/;

/* A selector reduced to what it matches when the page is in no state, with the states that
   removal took out and the prefix through the construct that carried the first of them.

   Deleting the pseudo-class alone is not enough. `.root:where(:focus-visible,[x])` becomes
   `:where(,[x])`, which does not parse, and tidying that to `:where([x])` is worse: it is
   valid and matches a different population, so the element that will take the focus ring is
   never found. A nested list is a logical OR, so the branch carrying the state is the way in
   this rule describes and its siblings are other ways in, which the base-state path already
   records. Following that branch and dropping the rest reduces the selector to `.root` — the
   element the ring lands on. A construct the branch empties goes with it. */
const resting = selector => {
  const states = [];
  let ownerEnd = -1;
  let out = '';
  let index = 0;
  const carried = () => { if (ownerEnd < 0 && states.length) ownerEnd = out.length; };
  while (index < selector.length) {
    const char = selector[index];
    if (char === '\\') { out += selector.slice(index, index + 2); index += 2; continue; }
    if (char === '"' || char === "'") {
      let end = index + 1;
      while (end < selector.length && selector[end] !== char) end += selector[end] === '\\' ? 2 : 1;
      out += selector.slice(index, end + 1);
      index = end + 1;
      continue;
    }
    const name = char === ':' ? PSEUDO_NAME.exec(selector.slice(index)) : null;
    if (!name) { out += char; index++; continue; }
    const after = index + name[0].length;
    if (selector[after] !== '(') {
      if (STATE_NAME.test(selector.slice(index))) states.push(name[0]);
      else out += name[0];
      carried();
      index = after;
      continue;
    }
    const close = closingParen(selector, after);
    const end = close < 0 ? selector.length - 1 : close;
    let kept = null;
    for (const branch of selectorMembers(selector.slice(after + 1, end))) {
      const inner = resting(branch);
      if (!inner.states.length) continue;
      states.push(...inner.states);
      kept = inner.text;
      break;
    }
    if (kept === null) out += selector.slice(index, end + 1);
    else if (kept) out += `${name[0]}(${kept})`;
    carried();
    index = end + 1;
  }
  const text = out.trim().replace(/\s+/g, ' ');
  return { text, states, owner: ownerEnd < 0 ? '' : out.slice(0, ownerEnd).trim() };
};
const restingSelector = selector => resting(selector).text;

/* The pseudo-element a member generates a box for, the subject it generates it on, and
   whatever trails it. The name may take an argument — `::part(a,b)`, `::slotted(.a,.b)` —
   so where it ends is the same balanced question as everything else here. */
const generatedBoxOf = member => {
  const at = scanValue(member, 0, (char, depth, index) =>
    char === ':' && !depth && member[index + 1] === ':');
  const name = at < 0 ? null : PSEUDO_NAME.exec(member.slice(at));
  if (!name) return null;
  const after = at + name[0].length;
  const close = member[after] === '(' ? closingParen(member, after) : after - 1;
  const end = close < 0 ? member.length - 1 : close;
  return { suffix: name[0], subject: member.slice(0, at), tail: member.slice(end + 1) };
};

/* A member with every generated box taken off it, which is what a stage asking about the
   originating element rather than the box wants. */
const withoutGeneratedBoxes = member => {
  let text = member;
  for (let box = generatedBoxOf(text); box; box = generatedBoxOf(text)) {
    text = box.subject + box.tail;
  }
  return text;
};

/* Where a state lives relative to the element the rule styles.

   Selectors defines exactly one relational pseudo-class, so there are only two answers: the
   state is held by something the subject contains, or by the subject itself or one of its
   ancestors. Which of the two is read off the selector; which elements each names is asked
   of the engine, never derived from more text. */
const stateRelation = subject => {
  for (let index = 0; index <= subject.length; ) {
    const at = scanValue(subject, index, (char, depth, offset) =>
      char === ':' && !depth && RELATIONAL.test(subject.slice(offset)));
    if (at < 0) break;
    const close = closingParen(subject, at + 4);
    const end = close < 0 ? subject.length - 1 : close;
    for (const branch of selectorMembers(subject.slice(at + 5, end))) {
      const inner = resting(branch);
      if (!inner.states.length) continue;
      const outer = resting(subject.slice(0, at) + subject.slice(end + 1));
      return {
        query: outer.text,
        holder: inner.text,
        contained: true,
        states: inner.states,
        tailStates: outer.states
      };
    }
    index = end + 1;
  }
  const whole = resting(subject);
  return {
    query: whole.text,
    holder: whole.owner,
    contained: false,
    states: whole.states,
    tailStates: resting(lastCompound(subject)).states
  };
};
