const styles = new Set();
const sheets = new Map();
// A shadow tree is a separate style scope: nothing adopted by the document reaches inside
// one, so every root that is opened has to be adopted into as well, and every sheet
// registered afterwards has to reach all of them. One cache of sheets serves every root,
// because a constructed sheet may be adopted by any number of them.
const roots = new Set([document]);
let finalized = false;

export function registerStyle(css) {
  styles.add(css);
  if (finalized) adopt(css);
}

export function adoptRegisteredStyles(initial = []) {
  initial.forEach(css => styles.add(css));
  finalized = true;
  styles.forEach(adopt);
}

export function adoptInto(root) {
  if (roots.has(root)) return;
  roots.add(root);
  root.adoptedStyleSheets = [...root.adoptedStyleSheets, ...sheets.values()];
}

function adopt(css) {
  if (sheets.has(css)) return;
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(css);
  sheets.set(css, sheet);
  roots.forEach(root => {
    root.adoptedStyleSheets = [...root.adoptedStyleSheets, sheet];
  });
}
