/* What the stages ask of a selector, on top of the character scan that reads it.

   Every question here is answered once and asked by three stages — the state capture, the
   activation probe and the generated-box scan — so a selector cannot be read two ways.
   Concatenated after `selector_scan.js`, whose scanner these readers are built from. */
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
  // The construct that carried the first state belongs to a compound, and the compound is
  // what the holder query has to name — `:focus-visible` may sit in the middle of one. So the
  // cut runs on to the next top-level combinator, and everything past it is the join the
  // author wrote between the holder and the subject.
  let end = ownerEnd;
  if (end >= 0) {
    const cut = scanValue(out, end, (char, depth) => !depth && ' >+~\t\n'.includes(char));
    end = cut < 0 ? out.length : cut;
  }
  return {
    text,
    states,
    owner: end < 0 ? '' : out.slice(0, end).trim(),
    rest: end < 0 ? '' : out.slice(end).replace(/\s+/g, ' ')
  };
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

   A state reaches its subject through a combinator or through the one relational
   pseudo-class, so the answer is the combinator the author wrote, or containment. Which of
   them is read off the selector; which elements each names is asked of the engine, never
   derived from more text.

   `AXIS` names the four combinators Selectors defines. A join carrying intermediate compounds
   is not one of them and falls back to the ancestor relation, which is the only thing a
   single relation can say about a reach that took several steps. */
const AXIS = {
  ' ': 'ancestor',
  '>': 'parent',
  '+': 'previous_sibling',
  '~': 'preceding_sibling'
};
const joinOf = rest => {
  if (!rest) return '';
  const subject = lastCompound(rest);
  return rest.slice(0, rest.lastIndexOf(subject)).trim() || ' ';
};

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
        rest: '',
        axis: 'contained',
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
    rest: whole.rest,
    axis: AXIS[joinOf(whole.rest)] || 'ancestor',
    contained: false,
    states: whole.states,
    tailStates: resting(lastCompound(subject)).states
  };
};
