// A scripted DOM that models exactly the two platform guarantees this rule turns on, and
// nothing else: `element.matches` answers about the element itself wherever it lives, while
// `document.querySelectorAll` is confined to one node tree and cannot cross a shadow
// boundary. Modelling anything more would test a selector engine this file did not write;
// modelling anything less would let both reaches coincide, which is the whole defect.
//
// `location.href` and `document.baseURI` are supplied separately because they are separate
// values: HTML resolves a relative reference against the document base URL, which a
// `<base href>` overrides, while the location never sees it. They are equal on every page
// without a `<base>`, so a harness that derived one from the other could not express the
// input where the distinction is the whole point.
globalThis.location = { href: '__BASE__' };

const element = spec => {
  const node = {
    tagName: (spec.tag || 'div').toUpperCase(),
    attributes: Object.entries(spec.attributes || {}).map(([name, value]) => ({ name, value })),
    style: spec.style || {},
    pseudos: spec.pseudos || {},
    assetBearing: Boolean(spec.assetBearing),
    children: (spec.children || []).map(element),
    shadowRoot: null
  };
  node.matches = () => node.assetBearing;
  if (spec.shadow) node.shadowRoot = { mode: 'open', children: spec.shadow.map(element) };
  return node;
};

const tree = element(__TREE__);

// Slotting never moves a node, so the light tree is the host's own children — the reach a
// document-rooted query has, and the reach it is limited to.
const lightTree = node => [node, ...node.children.flatMap(lightTree)];
globalThis.document = {
  baseURI: '__DOCUMENT_BASE__',
  querySelectorAll: () => lightTree(tree).filter(node => node.assetBearing)
};

__ASSET_ATTRIBUTES__

// The walk exactly as `page_capture.js` performs it: an element, then its children, then its
// shadow children. This is the reach every stage is supposed to share.
const nodes = [];
const walk = node => {
  nodes.push({
    attributes: recreateAttributes(node, node.tag),
    style: node.style,
    pseudos: node.pseudos
  });
  for (const child of node.children) walk(child);
  if (node.shadowRoot) for (const child of node.shadowRoot.children) walk(child);
};
walk(tree);

// The projection exactly as `page_capture.js` performs it: resolve each rule's references
// against its own sheet, then read the asset set out of those very strings.
const { texts: cssRuleTexts, urls: assets } = recreateCssAssets(nodes, __CSS_RULES__);

console.log(JSON.stringify({
  assets: [...assets].sort(),
  rules: cssRuleTexts,
  recorded: nodes.map(node => node.attributes)
}));
