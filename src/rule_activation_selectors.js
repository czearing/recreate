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

// A member is read right to left: the subject must match the last compound, and every
// compound before it must match some ancestor of it.
const memberMatches = (member, element) => {
  const compounds = member.trim().split(/\s+/);
  if (!compoundMatches(compounds.pop(), element)) return false;
  let subject = element;
  while (compounds.length) {
    const compound = compounds.pop();
    const ancestor = elements.find(node => contains(node, subject) && compoundMatches(compound, node));
    if (!ancestor) return false;
    subject = ancestor;
  }
  return true;
};

const matchesSelector = (selector, element) =>
  doubleMembers(selector).some(member => memberMatches(member, element));
