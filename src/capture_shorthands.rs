//! Which longhands an authored declaration block sets, and to what, as the engine divides them.
//!
//! A capture enumerates longhands, so every stage that compares authored text against a sample
//! has to know what an author's shorthand name stands for. Reading that off the names answers
//! only the half of the question that needs no grammar: which longhands a shorthand *may* set.
//! The other half — which share each one gets — is a per-family grammar, two lengths on a box
//! being one per axis while two words on `font` are a size and a family, and there is no
//! spelling of that rule that is not a table of families. A table answers for the families
//! somebody listed and silently withholds the rest, which is the same as withholding at random.
//!
//! The engine already performed the division. CSSOM stores a declaration block as longhands —
//! a shorthand is a parsing convenience and is not retained — so `rule.style` enumerates
//! exactly the longhands the author's declarations set and `getPropertyValue` returns each
//! share, already serialised in the same vocabulary a computed sample uses. Reading it is
//! total, exact, and costs one property read per longhand of rules the page already parsed.
//!
//! Only blocks that actually contain a shorthand are recorded, and that too is the engine's
//! answer rather than a list: a block is a shorthand block when it sets a longhand it does not
//! name. A block whose every stored longhand appears in its own text tells the later stages
//! nothing they cannot read from the text, so recording it would be size without information.
//!
//! Two shares are refused. A CSS-wide keyword is not a value — it is an instruction to the
//! cascade, and the omitted components of a shorthand are all reported as `initial`, which is
//! most of what a shorthand stores and none of what it says. And a share is kept even when it
//! is empty: a value holding `var()` divides into longhands the engine cannot yet settle, and
//! it says so by reporting them present and blank. That blank is the difference between "the
//! author wrote no arm" and "the author wrote one nothing here can read", which is exactly the
//! distinction a later stage must not collapse before deciding to delete a declaration.
pub const SOURCE: &str = r#"
  const cssWideShare = /^(initial|inherit|unset|revert|revert-layer)$/i;
  const shorthandBlocks = new Map();
  const recordShorthandBlock = (style, base) => {
    const text = (style.cssText || '').trim();
    if (!text || shorthandBlocks.has(text)) return;
    const shares = {};
    for (const property of Array.from(style)) {
      const share = style.getPropertyValue(property);
      if (cssWideShare.test(share.trim())) continue;
      shares[property] = share;
    }
    // The engine's own answer to "is there a shorthand here": a longhand it stored that the
    // author did not name. Reading the text for `:` positions would be a second declaration
    // parser, and the artifact is entitled to exactly one.
    if (!Object.keys(shares).some(property => !text.includes(property))) return;
    shorthandBlocks.set(text, { text, base, shares });
  };
"#;
