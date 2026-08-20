// The double's selector engine: which elements a selector picks out. Deliberately independent
// of the capture's own reader, because a fixture that borrowed the reader under test could not
// fail when that reader is wrong. `elements` and `ancestry` come from
// `rule_activation_harness.js`, which is concatenated after this file and reached only from
// inside these bodies.
const doubleParen = (text, open) => {
  let depth = 0;
  for (let index = open; index < text.length; index++) {
    if (text[index] === '(') depth++;
    else if (text[index] === ')' && --depth === 0) return index;
  }
  return -1;
};
const doubleMembers = text => {
  const members = [];
  let depth = 0;
  let start = 0;
  for (let index = 0; index < text.length; index++) {
    if (text[index] === '(' || text[index] === '[') depth++;
    else if (text[index] === ')' || text[index] === ']') depth--;
    else if (text[index] === ',' && !depth) {
      members.push(text.slice(start, index).trim());
      start = index + 1;
    }
  }
  members.push(text.slice(start).trim());
  return members.filter(Boolean);
};

// Which elements a selector picks out. The double answers this the way an engine does, and
// refuses what an engine refuses: text that is not a selector throws rather than matching
// nothing, because a stage that hands the engine a fragment cut out of the middle of a
// selector is told so by the throw and by nothing else.
//
// A compound is a run of `.class`, `[attribute]`, `[attribute=value]`, a type name and the
// logical pseudo-classes, and compounds are joined by descendant combinators. A construct
// outside that is modelled as matching nothing, which is what an engine does with a selector
// no element satisfies. A delimiter with no partner is different in kind: no engine reads it
// as a selector at all, so neither does this.
const compoundMatches = (compound, element) => {
  let rest = compound.trim();
  if (rest === '*') return true;
  if (!rest) throw new Error('a selector member is empty');
  let matched = true;
  while (rest) {
    if (rest[0] === ')' || rest[0] === ',' || rest[0] === '(') {
      throw new Error(`unbalanced selector: ${compound}`);
    }
    const relational = /^:has\(/.exec(rest);
    if (relational) {
      const close = doubleParen(rest, relational[0].length - 1);
      if (close < 0) throw new Error(`unbalanced selector: ${compound}`);
      matched = matched && doubleMembers(rest.slice(relational[0].length, close))
        .some(branch => reaches(segments(branch), element));
      rest = rest.slice(close + 1);
      continue;
    }
    const logical = /^:(is|where|not)\(/.exec(rest);
    if (logical) {
      const open = logical[0].length - 1;
      const close = doubleParen(rest, open);
      if (close < 0) throw new Error(`unbalanced selector: ${compound}`);
      const any = doubleMembers(rest.slice(open + 1, close))
        .some(branch => compoundMatches(branch, element));
      matched = matched && (logical[1] === 'not' ? !any : any);
      rest = rest.slice(close + 1);
      continue;
    }
    const className = /^\.([\w-]+)/.exec(rest);
    if (className) {
      matched = matched && element.classes.includes(className[1]);
      rest = rest.slice(className[0].length);
      continue;
    }
    const attribute = /^\[([\w-]+)(?:=([^\]]*))?\]/.exec(rest);
    if (attribute) {
      const held = (element.attributes || {})[attribute[1]];
      matched = matched && (attribute[2] === undefined ? held !== undefined : held === attribute[2]);
      rest = rest.slice(attribute[0].length);
      continue;
    }
    const other = /^(?:[\w-]+|#[\w-]+|::?[\w-]+(?:\([^()]*\))?)/.exec(rest);
    if (!other) throw new Error(`unbalanced selector: ${compound}`);
    matched = matched && other[0] === element.tag;
    rest = rest.slice(other[0].length);
  }
  return matched;
};

const contains = (ancestor, element) => element.path.startsWith(`${ancestor.path}/`);

// The DOM the double models, read off the paths the scene declares. Document order is the
// order the scene lists its elements in, which is what decides sibling direction.
const parentPath = path => path.slice(0, path.lastIndexOf('/'));
const siblings = element => elements.filter(node => parentPath(node.path) === parentPath(element.path));
const preceding = element => {
  const row = siblings(element);
  return row.slice(0, row.indexOf(element));
};
const following = element => {
  const row = siblings(element);
  return row.slice(row.indexOf(element) + 1);
};
const parent = element => elements.filter(node => node.path === parentPath(element.path));

// Selectors defines four combinators, and each names one axis in each direction. `LEFT` is
// what a complex selector walks when it reads right to left; `RIGHT` is what a relative
// selector inside `:has()` reaches from its anchor. Nothing else distinguishes them.
const LEFT = {
  ' ': element => elements.filter(node => contains(node, element)),
  '>': parent,
  '+': element => preceding(element).slice(-1),
  '~': preceding
};
const RIGHT = {
  ' ': element => elements.filter(node => contains(element, node)),
  '>': element => elements.filter(node => parentPath(node.path) === element.path),
  '+': element => following(element).slice(0, 1),
  '~': following
};

// A member cut into compounds, each carrying the combinator that precedes it. A combinator
// only separates compounds at the top level, so the space in `[title="a b"]` divides nothing.
// A leading combinator leaves the first compound's join set, which is what makes a relative
// selector relative; an absent one is the descendant combinator.
const segments = member => {
  const parts = [];
  let depth = 0;
  let text = '';
  let join = null;
  const push = () => {
    parts.push({ join, compound: text });
    text = '';
    join = null;
  };
  for (const character of member) {
    if (character === '(' || character === '[') depth++;
    else if (character === ')' || character === ']') depth--;
    if (!depth && ' \t\n>+~'.includes(character)) {
      if (text) push();
      if ('>+~'.includes(character)) join = character;
      else if (join === null) join = ' ';
      continue;
    }
    text += character;
  }
  if (text) push();
  if (parts.length && parts[0].join === null) parts[0].join = ' ';
  return parts;
};

// Whether the anchor reaches this relative selector, walking left to right.
const reaches = (parts, anchor) => {
  const [head, ...rest] = parts;
  return RIGHT[head.join](anchor).some(
    node => compoundMatches(head.compound, node) && (!rest.length || reaches(rest, node))
  );
};

// A member is read right to left: the subject matches the last compound, and each compound
// before it matches some element on the axis its combinator names. The search backtracks,
// because the first candidate on an axis need not be the one that lets the rest of the
// member match.
const memberMatches = (member, element) => {
  const parts = segments(member.trim());
  const subject = parts.pop();
  if (!compoundMatches(subject.compound, element)) return false;
  const reachedBy = (remaining, join, node) => {
    if (!remaining.length) return true;
    const head = remaining[remaining.length - 1];
    return LEFT[join](node).some(
      candidate =>
        compoundMatches(head.compound, candidate) &&
        reachedBy(remaining.slice(0, -1), head.join, candidate)
    );
  };
  return reachedBy(parts, subject.join, element);
};

const matchesSelector = (selector, element) =>
  doubleMembers(selector).some(member => memberMatches(member, element));
