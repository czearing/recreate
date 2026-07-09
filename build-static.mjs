import fs from 'node:fs';
import path from 'node:path';

const specDir = 'C:/Code/onenote-spec';
const buildDir = 'C:/Code/onenote-build';
const spec = JSON.parse(fs.readFileSync(path.join(specDir, 'spec.json'), 'utf8'));
const cap = spec.captures[0];
const nodes = cap.nodes;

// Build parent->children map
const childrenOf = new Map();
const nodeByPath = new Map();
for (const node of nodes) {
  nodeByPath.set(node.path, node);
  const key = node.parentPath || '';
  if (!childrenOf.has(key)) childrenOf.set(key, []);
  childrenOf.get(key).push(node);
}

const VOID_TAGS = new Set(['area','base','br','col','embed','hr','img','input','link','meta','param','source','track','wbr']);

function styleToStr(style) {
  if (!style) return '';
  return Object.entries(style)
    .filter(([k, v]) => v !== '' && v != null)
    .map(([k, v]) => `${k}:${v}`)
    .join(';');
}

function attrsToStr(attrs, extraStyle) {
  if (!attrs) return '';
  const parts = [];
  for (const [k, v] of Object.entries(attrs)) {
    if (k === 'style') continue; // handled separately
    if (v === null || v === undefined || v === '') continue;
    parts.push(`${k}="${String(v).replace(/"/g, '&quot;')}"`);
  }
  if (extraStyle) parts.push(`style="${extraStyle.replace(/"/g, '&quot;')}"`);
  return parts.length ? ' ' + parts.join(' ') : '';
}

function renderNode(node) {
  const tag = node.tag;
  if (tag === '#document' || tag === 'html' || tag === 'head') return '';
  if (tag === '#text') {
    const t = (node.text || '').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
    return t;
  }
  
  const children = childrenOf.get(node.path) || [];
  const style = styleToStr(node.style);
  const attrs = attrsToStr(node.attrs, style);
  
  if (VOID_TAGS.has(tag)) return `<${tag}${attrs}>`;
  
  const inner = children.map(renderNode).join('');
  return `<${tag}${attrs}>${inner}</${tag}>`;
}

// Find body node
const bodyNode = nodes.find(n => n.tag === 'body');

// Collect all stylesheets
const stylesheetsDir = path.join(specDir, 'stylesheets');
const cssFiles = fs.readdirSync(stylesheetsDir).filter(f => f.endsWith('.css')).sort();
const cssLinks = cssFiles.map(f => `  <link rel="stylesheet" href="./stylesheets/${f}">`).join('\n');

// Copy stylesheets to build
fs.mkdirSync(path.join(buildDir, 'stylesheets'), { recursive: true });
for (const f of cssFiles) {
  fs.copyFileSync(path.join(stylesheetsDir, f), path.join(buildDir, 'stylesheets', f));
}

const bodyChildren = (childrenOf.get(bodyNode?.path) || []).map(renderNode).join('\n');

const html = `<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Notebooks</title>
${cssLinks}
  <style>
    html, body { margin: 0; padding: 0; background: rgb(226,226,226); font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif; }
  </style>
</head>
<body>
${bodyChildren}
</body>
</html>`;

fs.mkdirSync(buildDir, { recursive: true });
fs.writeFileSync(path.join(buildDir, 'index.html'), html);
console.log('Written', html.length, 'chars');
console.log('Body nodes rendered:', (bodyNode ? (childrenOf.get(bodyNode.path)||[]).length : 0));
