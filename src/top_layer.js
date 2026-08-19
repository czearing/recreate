/* Why the engine was painting an element from the top layer, or nothing if it was not.

   The top layer is a user-agent list, not a property of any element, and nothing in the
   document declares membership of it. Three calls put an element there and they are not
   variants of one thing: they differ in what they do to the rest of the page, they are
   selected by three different pseudo-classes, and — the reason this is a reason and not a
   flag — a recreation puts an element back by making the same call the page made, so the
   record has to say which call that was.

   Recording inertness instead is what loses the largest member. `:modal` is specified as
   excluding all interaction with everything outside the element, which is true of a dialog
   opened with `showModal()` and of the fullscreen element and deliberately false of a
   popover, whose whole purpose is that the page behind it stays live. So the top layer is
   strictly wider than `:modal`, and an open popover answering that question records `false`
   — not "unknown", but a positive claim that it was an ordinary in-flow box.

   Ordered by how specific the answer is rather than by preference. `:modal` matches the
   fullscreen element as well as a modal dialog, so fullscreen is asked first or it would be
   reported as something a recreation may call `showModal()` on. A popover is never `:modal`,
   so its position is free; it leads because it is the common case.

   Fullscreen is recorded and never replayed. `requestFullscreen()` requires transient user
   activation, which a recreation loading itself does not have, so there is no call to make —
   but the record still has to distinguish it, or a later stage reading "in the top layer"
   would replay it as a dialog. Naming the reason is what lets one field carry a promotion
   with no replay beside two with different ones. */
const TOP_LAYER_REASONS = [
  [':popover-open', 'popover'],
  [':fullscreen', 'fullscreen'],
  [':modal', 'modal']
];
const recreateTopLayer = element => {
  for (const [selector, reason] of TOP_LAYER_REASONS) {
    try {
      if (element.matches(selector)) return reason;
    } catch (unsupported) {
      /* An engine that does not know a selector cannot have promoted anything for that
         reason, so the remaining ones are still worth asking. Throwing here would cost the
         page every promotion, including the ones the engine does support. */
    }
  }
  return '';
};
