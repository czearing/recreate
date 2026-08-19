//! Which of an element's properties an authored condition decided, as the engine answers it.
//!
//! A capture reads a page with its conditions on whichever branch they happened to be, so
//! every declaration it records is one branch of a rule the author wrote two arms for. The
//! emitter re-publishes the condition, and must therefore take that branch back out of the
//! unconditional rule — but only where a condition really put it there, because withdrawing
//! a value nothing puts back deletes it from the recreation.
//!
//! Proving it from the text cannot work. A conditional declaration is written in the
//! specified-value vocabulary and a sample is serialised in the computed one, so `0.5em`,
//! `5%`, `calc()`, `min()`, `10cqw`, `1lh` and every named colour compare unequal to the
//! very sample they produced. Any repair spelled as "resolve this unit first" is a table of
//! unit families that answers for the ones somebody listed and withholds every one added
//! after — and no table can resolve a percentage, which needs a containing block, or a
//! container unit, which needs the query container's used size.
//!
//! The same argument answers the *names*. A conditional block declaring only a custom
//! property decides whatever longhands read that token — through nested `var()`, through a
//! fallback, through a registered initial value, on this element or on any descendant the
//! token inherits to — and none of those longhands is spelled anywhere in the block. The set
//! of names a block states and the set of properties it decides are therefore different
//! sets, so neither the blocks to withdraw nor the properties to watch may be derived from
//! what a block happens to say. A block is withdrawn because the emitter re-publishes its
//! rule, and the properties watched are the ones the node will *bake*, which is exactly the
//! set the recreation can be wrong about.
//!
//! The engine already made the decision, and the page is still open. Withdrawing the
//! declaration blocks of exactly the rules the emitter re-publishes under a condition, and
//! reading the affected properties again, reports which of them a condition decided — for
//! every unit, function and keyword at once, including ones this repository has no name for.
//! The rules are withdrawn by emptying their blocks rather than by deleting them, so the
//! sheet's rule order, every selector, and the CSSOM node identity the walk holds are all
//! untouched, and restoring is the assignment back. It runs after the rule texts have been
//! recorded, because a rule's text is the block being emptied.
//!
//! Withdrawing every condition at once reports *that* a condition decided a property, not
//! *which one*, and the emitter has to put the override back under one particular prelude.
//! So the pass runs once globally to find the readings in play, then once per condition
//! chain over only those — never once per element and never once per property, because the
//! global pass has already reduced the page to the readings a condition can still move.
//!
//! The withdrawal is done with the affected sheets switched off, and each sheet's prior
//! switch restored after. A live sheet rebuilds its rule data every time one of its
//! declarations is handed a new text, so emptying its rules one at a time is quadratic in
//! the rules it holds: on github.com that is 32454 blocks and 42.6 seconds, against 19
//! milliseconds for the same blocks emptied while their 23 sheets are off. Every read
//! happens with the sheets back on, so what is measured is unchanged.
pub fn source() -> String {
    format!("{}{BODY}", crate::authored_condition_kind::js_source())
}

const BODY: &str = r#"
  const conditionDecided = (entries, pairs) => {
    const texts = new Map();
    const groups = new Map();
    const sheets = new Map();
    for (const entry of entries) {
      // A rule with no selector reaches no element, so no element's value is its doing.
      // `@font-face`, `@property`, `@position-try` and every definition rule the platform
      // adds next all carry a declaration block that names descriptors rather than an
      // element's properties, and none of them is the emitter's to rewrite.
      if (!entry.active || !entry.rule.selectorText) continue;
      // The chain the emitter re-publishes this rule under, which is also the key its
      // override is credited to. Empty for a gate — `@supports` — which the engine rather
      // than the document answers and the recreation therefore bakes away, and empty for a
      // chain with no false branch, where there is no second arm to restore.
      const opening = conditionOpening(entry.carriers);
      if (!opening) continue;
      const block = entry.rule.style;
      if (!block || texts.has(block)) continue;
      texts.set(block, block.cssText);
      if (!groups.has(opening)) groups.set(opening, []);
      groups.get(opening).push(block);
      const sheet = entry.rule.parentStyleSheet;
      if (sheet && !sheets.has(sheet)) sheets.set(sheet, sheet.disabled);
    }
    if (!texts.size) return;
    // The properties each node will bake, which is the whole of what the recreation can
    // state wrongly. Asking the blocks what to watch instead misses every property a token
    // decides, and watches properties no generated class ever carries.
    const watched = pairs.map(([, node]) => Object.keys(node.style || {}));
    const read = index => {
      const computed = getComputedStyle(pairs[index][0]);
      return watched[index].map(name => computed.getPropertyValue(name));
    };
    const readAll = () => pairs.map((pair, index) => read(index));
    // A sheet is switched off only while its blocks are being rewritten, and every reading
    // below happens with it back on.
    const rewrite = write => {
      for (const sheet of sheets.keys()) sheet.disabled = true;
      try { write(); } finally {
        for (const [sheet, disabled] of sheets) sheet.disabled = disabled;
      }
    };
    const withdraw = blocks => rewrite(() => {
      for (const block of blocks) {
        try { block.cssText = ''; } catch { texts.delete(block); }
      }
    });
    const putBack = blocks => rewrite(() => {
      for (const block of blocks) {
        const text = texts.get(block);
        if (text === undefined) continue;
        try { block.cssText = text; } catch {}
      }
    });
    const before = readAll();
    const every = Array.from(texts.keys());
    withdraw(every);
    let after;
    try { after = readAll(); } finally { putBack(every); }
    // Every reading a condition can still move, with the arm the unconditional cascade owes
    // recorded beside it. The engine settles both arms, so a value the author expressed only
    // through a token, a percentage or a container unit is as answerable as a literal.
    const moved = [];
    pairs.forEach(([, node], index) => {
      const slots = watched[index]
        .map((name, slot) => slot)
        .filter(slot => after[index][slot] !== before[index][slot]);
      if (!slots.length) return;
      node.condition_base = {};
      for (const slot of slots) node.condition_base[watched[index][slot]] = after[index][slot];
      moved.push([index, slots]);
    });
    if (!moved.length) return;
    const credit = (index, slots, opening) => {
      if (!slots.length) return;
      const node = pairs[index][1];
      node.condition_decided = node.condition_decided || {};
      node.condition_decided[opening] = slots.map(slot => watched[index][slot]);
    };
    const openings = Array.from(groups.keys());
    // One chain answers for everything that moved, so the attributing passes below would
    // only re-derive what the global pass has already settled.
    if (openings.length === 1) {
      for (const [index, slots] of moved) credit(index, slots, openings[0]);
      return;
    }
    for (const opening of openings) {
      const blocks = groups.get(opening);
      withdraw(blocks);
      try {
        for (const [index, slots] of moved) {
          const computed = getComputedStyle(pairs[index][0]);
          credit(
            index,
            slots.filter(
              slot => computed.getPropertyValue(watched[index][slot]) !== before[index][slot]
            ),
            opening
          );
        }
      } finally { putBack(blocks); }
    }
  };
  conditionDecided(ruleEntries, elementNodes);
"#;
