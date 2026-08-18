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
//! The engine already made the decision, and the page is still open. Withdrawing the
//! declaration blocks of exactly the rules the emitter re-publishes under a condition, and
//! reading the affected properties again, reports which of them a condition decided — for
//! every unit, function and keyword at once, including ones this repository has no name for.
//! The rules are withdrawn by emptying their blocks rather than by deleting them, so the
//! sheet's rule order, every selector, and the CSSOM node identity the walk holds are all
//! untouched, and restoring is the assignment back. It runs after the rule texts have been
//! recorded, because a rule's text is the block being emptied.
//!
//! The reach is the emitter's, not the engine's: a condition also decides properties on
//! elements its selector reaches through an ancestor or a compound the node only partly
//! satisfies, which the emitter cannot rewrite and must not withdraw. This records the
//! engine's answer and leaves that intersection to the stage that knows its own reach.
//!
//! The withdrawal is done with the affected sheets switched off, and each sheet's prior
//! switch restored after. A live sheet rebuilds its rule data every time one of its
//! declarations is handed a new text, so emptying its rules one at a time is quadratic in
//! the rules it holds: on github.com that is 32454 blocks and 42.6 seconds, against 19
//! milliseconds for the same blocks emptied while their 23 sheets are off. Every read
//! happens with the sheets back on, so what is measured is unchanged.
pub const SOURCE: &str = r#"
  const conditionDecided = (entries, pairs) => {
    const blocks = new Map();
    const sheets = new Map();
    const properties = new Set();
    for (const entry of entries) {
      // Carriers are the conditions the recreation re-emits verbatim. A gate — `@supports`
      // — is answered by the engine rather than the document and is baked away, so its
      // branch is the recreation's unconditional value and withdrawing it deletes one.
      if (!entry.active || !entry.carriers.length) continue;
      // A rule with no selector reaches no element, so no element's value is its doing.
      // `@font-face`, `@property`, `@position-try` and every definition rule the platform
      // adds next all carry a declaration block that names descriptors rather than an
      // element's properties, and none of them is the emitter's to rewrite.
      if (!entry.rule.selectorText) continue;
      const block = entry.rule.style;
      if (!block || blocks.has(block)) continue;
      let named = false;
      // The block's own enumeration, which is the longhands the engine kept rather than the
      // shorthands the author wrote. Custom properties are left out: their effect reaches an
      // element through whatever longhand reads them, which is what gets measured instead.
      for (let slot = 0; slot < block.length; slot++) {
        const name = block.item(slot);
        if (name && !name.startsWith('--')) {
          properties.add(name);
          named = true;
        }
      }
      if (!named) continue;
      blocks.set(block, block.cssText);
      const sheet = entry.rule.parentStyleSheet;
      if (sheet && !sheets.has(sheet)) sheets.set(sheet, sheet.disabled);
    }
    if (!blocks.size) return;
    const names = Array.from(properties);
    const read = () => pairs.map(([element]) => {
      const computed = getComputedStyle(element);
      return names.map(name => computed.getPropertyValue(name));
    });
    // A sheet is switched off only while its blocks are being rewritten, and every reading
    // below happens with it back on.
    const rewrite = write => {
      for (const sheet of sheets.keys()) sheet.disabled = true;
      try { write(); } finally {
        for (const [sheet, disabled] of sheets) sheet.disabled = disabled;
      }
    };
    const before = read();
    rewrite(() => {
      for (const block of blocks.keys()) {
        try { block.cssText = ''; } catch { blocks.delete(block); }
      }
    });
    try {
      const after = read();
      pairs.forEach(([, node], index) => {
        const decided = names.filter((name, slot) => after[index][slot] !== before[index][slot]);
        if (decided.length) node.condition_decided = decided;
      });
    } finally {
      rewrite(() => {
        for (const [block, text] of blocks) {
          try { block.cssText = text; } catch {}
        }
      });
    }
  };
  conditionDecided(ruleEntries, elementNodes);
"#;
