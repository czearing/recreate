// Builds the elements the blocking-overlay rule is asked about, on top of the shared style
// double. Each entry names the element under test plus the ancestors above it, outermost
// first, because a declaration on an ancestor is the only shape the defect appears in.

globalThis.buildOverlayFixture = entries => entries.map(entry => {
  let parent = null;
  for (const declarations of entry.ancestors || []) {
    parent = recreateStyled({ parent, declarations });
  }
  const element = recreateStyled({ parent, declarations: entry.style });
  element.getBoundingClientRect = () => entry.rect;
  return element;
});
