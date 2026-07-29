const styles = new Set();
const sheets = new Map();
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

function adopt(css) {
  if (sheets.has(css)) return;
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(css);
  document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
  sheets.set(css, sheet);
}
