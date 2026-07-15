import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const VOID_TAGS = new Set([
  'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link',
  'meta', 'param', 'source', 'track', 'wbr',
]);
const TRANSPARENT_IMAGE =
  'data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    if (!arg.startsWith('--')) continue;
    const next = argv[index + 1];
    args[arg.slice(2)] = next && !next.startsWith('--') ? next : true;
  }
  return args;
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function escapeAttribute(value) {
  return escapeHtml(value).replace(/"/g, '&quot;');
}

function styleToString(style) {
  if (!style) return '';
  return Object.entries(style)
    .filter(([, value]) => value !== '' && value != null)
    .map(([property, value]) => `${property}:${value}`)
    .join(';');
}

const RESPONSIVE_STYLE_PROPERTIES = [
  'display', 'position', 'inset', 'insetBlock', 'insetInline',
  'width', 'height', 'minWidth', 'maxWidth', 'minHeight', 'maxHeight',
  'margin', 'padding', 'gap', 'rowGap', 'columnGap', 'flex',
  'flexDirection', 'flexWrap', 'justifyContent', 'alignItems', 'alignSelf',
  'order', 'gridTemplateColumns', 'gridTemplateRows', 'gridAutoFlow',
  'overflow', 'overflowX', 'overflowY', 'boxSizing',
];

function cssPropertyName(value) {
  return value.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
}

function responsiveStylesForState(state, specDir) {
  const entries = Object.entries(state?.evidenceByViewport || {});
  if (entries.length < 2) return [];
  const primaryFile = state.evidence && path.join(specDir, state.evidence);
  const primary = primaryFile && fs.existsSync(primaryFile)
    ? JSON.parse(fs.readFileSync(primaryFile, 'utf8'))
    : null;
  const primaryByPath = new Map(
    (primary?.nodes || []).map((node) => [node.path, node]),
  );
  return entries
    .filter(([, file]) => file !== state.evidence)
    .map(([, file]) => JSON.parse(
      fs.readFileSync(path.join(specDir, file), 'utf8'),
    ))
    .map((evidence) => ({
      maxWidth: evidence.viewport?.width,
      entries: (evidence.nodes || []).map((node) => {
        const desktop = primaryByPath.get(node.path);
        const style = {};
        for (const property of RESPONSIVE_STYLE_PROPERTIES) {
          const value = node.style?.[property];
          if (value == null || value === desktop?.style?.[property]) continue;
          style[cssPropertyName(property)] = value;
        }
        return Object.keys(style).length ? { path: node.path, style } : null;
      }).filter(Boolean),
    }))
    .filter((item) => item.maxWidth && item.entries.length);
}

function localPathForState(state, sourceUrl) {
  if (state.type === 'route') {
    try {
      const source = new URL(sourceUrl);
      const target = new URL(state.url);
      if (target.origin === source.origin) {
        const pathname = target.pathname.replace(/\/+$/, '');
        return `${pathname || '/'}${target.search}`;
      }
    } catch {}
  }
  return `/__recreate/state/${String(state.index).padStart(3, '0')}`;
}

function outputPathForState(state, localPath) {
  if (localPath.includes('?')) {
    return `__recreate/query/${String(state.index).padStart(3, '0')}`;
  }
  return localPath.replace(/^\/+|\/+$/g, '');
}

function normalizedWords(value) {
  return String(value || '')
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/[^a-z0-9]+/gi, ' ')
    .trim()
    .toLowerCase();
}

function inferredTriggerLabel(state) {
  const deliverableMatch = String(state.text || '').match(
    /deliverables for "([^"]+)"/i,
  );
  if (deliverableMatch) return deliverableMatch[1];
  if (typeof state.trigger === 'string' && state.trigger.trim()) {
    return state.trigger.trim();
  }
  try {
    const segments = new URL(state.url).pathname.split('/').filter(Boolean);
    const notebookIndex = segments.indexOf('notebook');
    if (notebookIndex >= 0 && segments[notebookIndex + 1]) {
      return segments[notebookIndex + 1]
        .split('-')
        .map((word) => word ? word[0].toUpperCase() + word.slice(1) : '')
        .join(' ');
    }
  } catch {}
  return '';
}

function nodeLabel(node) {
  return (
    node.attrs?.['aria-label'] ||
    node.ariaLabel ||
    node.text ||
    node.attrs?.title ||
    ''
  ).trim();
}

function findTriggerNode(state, nodes, behaviors = []) {
  if (state.triggerElement?.path) {
    const exact = nodes.find((node) => node.path === state.triggerElement.path);
    if (exact) return exact;
  }

  const label = inferredTriggerLabel(state);
  if (!label) return undefined;
  const normalizedLabel = normalizedWords(label);
  const candidates = nodes.filter((node) => {
    if (!node.visible || !node.tag || node.tag.startsWith('#')) return false;
    const candidateLabel = normalizedWords(nodeLabel(node));
    return (
      candidateLabel === normalizedLabel ||
      candidateLabel.startsWith(`${normalizedLabel} `)
    );
  });
  const behaviorMatches = behaviors.filter((behavior) => {
    const candidateLabel = normalizedWords(behavior.label);
    return (
      candidateLabel === normalizedLabel ||
      candidateLabel.startsWith(`${normalizedLabel} `)
    );
  });
  const nodeForBehavior = (behavior) =>
    behavior && nodes.find((node) => node.path === behavior.path);

  if (state.type === 'panel') {
    const behavior = behaviorMatches.find((item) => item.ariaHaspopup) ||
      behaviorMatches.find((item) => item.tag === 'button');
    const behaviorNode = nodeForBehavior(behavior);
    if (behaviorNode) return behaviorNode;
    return candidates.find((node) => node.attrs?.['aria-haspopup']) ||
      candidates.find((node) => node.tag === 'button') ||
      candidates[0];
  }
  if (/\/notebook\//.test(state.url || '')) {
    return candidates.find((node) => node.attrs?.['data-testid'] === 'notebook-card') ||
      candidates.find((node) => node.role === 'button') ||
      candidates[0];
  }
  const behaviorNode = nodeForBehavior(
    behaviorMatches.find((item) => item.tag === 'button') ||
    behaviorMatches[0],
  );
  if (behaviorNode) return behaviorNode;
  return candidates.find((node) => node.tag === 'button') || candidates[0];
}

function triggerRecord(state, node, sourceUrl) {
  if (!node) return undefined;
  return {
    stateIndex: state.index,
    type: state.type,
    target: localPathForState(state, sourceUrl),
    path: node.path,
    label: state.triggerElement?.label || inferredTriggerLabel(state) || nodeLabel(node),
    tag: state.triggerElement?.tag || node.tag,
    role: state.triggerElement?.role || node.role || node.attrs?.role || '',
    testId: state.triggerElement?.testId || node.attrs?.['data-testid'] || '',
  };
}

function attrsToString(attrs, style, sourceUrl, extra = {}) {
  const values = { ...(attrs || {}), ...extra };
  delete values.style;
  const parts = [];
  for (const [name, rawValue] of Object.entries(values)) {
    if (rawValue === null || rawValue === undefined || rawValue === '') continue;
    let value = rawValue;
    if ((name === 'href' || name === 'action' || name === 'formaction') && sourceUrl) {
      try {
        const resolved = new URL(String(value), sourceUrl);
        if (resolved.origin === new URL(sourceUrl).origin) {
          value = `${resolved.pathname}${resolved.search}${resolved.hash}`;
        }
      } catch {}
    }
    parts.push(`${name}="${escapeAttribute(value)}"`);
  }
  if (style) parts.push(`style="${escapeAttribute(style)}"`);
  return parts.length ? ` ${parts.join(' ')}` : '';
}

function createRenderer(nodes, triggers, sourceUrl) {
  const childrenOf = new Map();
  for (const node of nodes) {
    const parent = node.parentPath || '';
    const children = childrenOf.get(parent) || [];
    children.push(node);
    childrenOf.set(parent, children);
  }
  const triggerByPath = new Map(triggers.map((trigger) => [trigger.path, trigger]));

  const renderNode = (node) => {
    const tag = node.tag;
    if (tag === '#document' || tag === 'html' || tag === 'head') return '';
    if (tag === '#text') return escapeHtml(node.text || '');
    if (!tag || !/^[a-z][a-z0-9-]*$/.test(tag)) return '';

    const trigger = triggerByPath.get(node.path);
    const extra = trigger
      ? {
          'data-recreate-target': trigger.target,
          'data-recreate-state': String(trigger.stateIndex),
        }
      : {};
    const style = styleToString(node.style);
    const attrs = attrsToString(node.attrs, style, sourceUrl, extra);
    if (VOID_TAGS.has(tag)) return `<${tag}${attrs}>`;
    const inner = (childrenOf.get(node.path) || []).map(renderNode).join('');
    return `<${tag}${attrs}>${inner}</${tag}>`;
  };

  return { childrenOf, renderNode };
}

function sanitizeCapturedHtml(
  html,
  stateUrl,
  cssFiles = [],
  stateStylesheet = '',
) {
  let output = String(html || '');
  const capturedBase = output.match(
    /<base\b[^>]*href=["']([^"']+)["'][^>]*>/i,
  )?.[1];
  output = output.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '');
  output = output.replace(
    /<link\b[^>]*rel=["'](?:modulepreload|preload)["'][^>]*>/gi,
    '',
  );
  output = output.replace(
    /<link\b(?=[^>]*rel=["']stylesheet["'])[^>]*>/gi,
    '',
  );
  output = output.replace(/\s+crossorigin(?:=["'][^"']*["'])?/gi, '');
  output = output.replace(/\s+on[a-z]+\s*=\s*(["'])[\s\S]*?\1/gi, '');
  output = output.replace(
    /(<img\b[^>]*\bsrc=)(["'])blob:[^"']+\2/gi,
    `$1$2${TRANSPARENT_IMAGE}$2`,
  );

  let baseUrl;
  try {
    baseUrl = capturedBase
      ? new URL(capturedBase, stateUrl).href
      : new URL('.', stateUrl).href;
  } catch {
    baseUrl = '/';
  }
  if (/<base\b/i.test(output)) {
    output = output.replace(/<base\b[^>]*>/i, `<base href="${escapeAttribute(baseUrl)}">`);
  } else {
    output = output.replace(/<head([^>]*)>/i, `<head$1><base href="${escapeAttribute(baseUrl)}">`);
  }

  const additions = [
    '<meta name="recreate-static-state" content="true">',
    ...cssFiles.map(
      (file) =>
        `<link rel="stylesheet" data-recreate-href="/stylesheets/${escapeAttribute(file)}">`,
    ),
    stateStylesheet
      ? `<link rel="stylesheet" data-recreate-href="${escapeAttribute(stateStylesheet)}">`
      : '',
    '<script>(()=>{const script=document.createElement("script");script.src=location.origin+"/recreate-runtime.js";script.defer=true;document.head.appendChild(script)})()</script>',
  ].join('');
  output = output.replace(/<\/head>/i, `${additions}</head>`);
  return output;
}

function runtimeSource(manifest) {
  return `(() => {
  const manifest = ${JSON.stringify(manifest)};
  for (const stylesheet of document.querySelectorAll('link[data-recreate-href]')) {
    stylesheet.href = location.origin + stylesheet.dataset.recreateHref;
  }
  const elementForPath = path => {
    const selector = String(path || '').replace(/^doc\\(0\\)>/, '');
    if (!selector) return null;
    try {
      return document.querySelector(selector);
    } catch {
      return null;
    }
  };
  const responsive = manifest.responsiveByPath?.[location.pathname] || [];
  for (const viewport of responsive) {
    if (innerWidth > viewport.maxWidth) continue;
    for (const entry of viewport.entries) {
      const element = elementForPath(entry.path);
      if (!element) continue;
      for (const [property, value] of Object.entries(entry.style)) {
        element.style.setProperty(property, value, 'important');
      }
    }
  }
  const normalized = value => String(value || '')
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/[^a-z0-9]+/gi, ' ')
    .trim()
    .toLowerCase();
  const labelFor = element =>
    element.getAttribute('aria-label') ||
    element.getAttribute('title') ||
    element.innerText ||
    '';
  const matches = (element, trigger) => {
    if (trigger.tag && element.tagName.toLowerCase() !== trigger.tag) return false;
    if (trigger.role && (element.getAttribute('role') || '') !== trigger.role) return false;
    if (trigger.testId && (element.getAttribute('data-testid') || '') !== trigger.testId) return false;
    const actual = normalized(labelFor(element));
    const expected = normalized(trigger.label);
    if (!expected) return false;
    return (
      actual === expected ||
      actual.startsWith(expected) ||
      actual.endsWith(' ' + expected)
    );
  };
  const interactive = element => element?.closest?.(
    'a[href],button,[role="button"],[role="link"],[data-recreate-target],[tabindex]:not([tabindex="-1"])'
  );
  const interactiveElements = Array.from(document.querySelectorAll(
    'a[href],button,[role="button"],[role="link"],[tabindex]:not([tabindex="-1"])'
  ));
  for (const trigger of manifest.triggers) {
    const exact = elementForPath(trigger.path);
    const semantic = exact
      ? []
      : interactiveElements.filter(element => matches(element, trigger));
    for (const element of exact ? [exact] : semantic) {
      element.dataset.recreateTarget = trigger.target;
      element.dataset.recreateState = String(trigger.stateIndex);
    }
  }
  const navigateTo = target => {
    const localUrl = new URL(target, location.origin);
    history.pushState(null, '', location.href);
    location.assign(localUrl.href);
  };
  const navigate = element => {
    const direct = element.getAttribute('data-recreate-target');
    const trigger = direct
      ? { target: direct }
      : manifest.triggers.find(candidate => matches(element, candidate));
    if (trigger?.target) {
      navigateTo(trigger.target);
      return true;
    }
    const homeLabel = normalized(labelFor(element));
    if (/(^| )home($| )/.test(homeLabel)) {
      navigateTo('/');
      return true;
    }
    return false;
  };
  document.addEventListener('click', event => {
    const element = interactive(event.target);
    if (!element || !navigate(element)) return;
    event.preventDefault();
    event.stopImmediatePropagation();
  }, true);
  document.addEventListener('keydown', event => {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    const element = interactive(event.target);
    if (!element || !navigate(element)) return;
    event.preventDefault();
    event.stopImmediatePropagation();
  }, true);
})();\n`;
}

function serverSource(queryRoutes) {
  return `import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.dirname(fileURLToPath(import.meta.url));
const port = Number(process.env.PORT || process.argv[2] || 4317);
const queryRoutes = ${JSON.stringify(queryRoutes)};
const types = {
  '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
};

http.createServer((request, response) => {
  const url = new URL(request.url || '/', 'http://localhost');
  const mappedPath = queryRoutes[url.pathname + url.search];
  const pathname = decodeURIComponent(mappedPath || url.pathname);
  const relative = pathname === '/' ? 'index.html' : pathname.replace(/^\\/+/, '');
  const requested = path.resolve(root, relative);
  const candidates = [
    requested,
    path.join(requested, 'index.html'),
    path.join(root, '404.html'),
  ];
  const file = candidates.find(candidate =>
    candidate.startsWith(root + path.sep) && fs.existsSync(candidate) && fs.statSync(candidate).isFile()
  );
  if (!file) {
    response.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
    return response.end('Not found');
  }
  response.writeHead(200, {
    'Content-Type': types[path.extname(file).toLowerCase()] || 'application/octet-stream',
    'Cache-Control': 'no-store',
  });
  fs.createReadStream(file).pipe(response);
}).listen(port, '127.0.0.1', () => {
  console.log('Serving recreate build at http://127.0.0.1:' + port + '/');
});
`;
}

export function buildStatic({ specDir, buildDir }) {
  const resolvedSpecDir = path.resolve(specDir);
  const resolvedBuildDir = path.resolve(buildDir);
  const spec = JSON.parse(
    fs.readFileSync(path.join(resolvedSpecDir, 'spec.json'), 'utf8'),
  );
  const captureRecord = spec.captures?.[0];
  const capture = captureRecord?.file
    ? JSON.parse(
        fs.readFileSync(path.join(resolvedSpecDir, captureRecord.file), 'utf8'),
      )
    : captureRecord;
  if (!capture?.nodes?.length) {
    throw new Error('The specification has no captured nodes to build.');
  }

  fs.rmSync(resolvedBuildDir, { recursive: true, force: true });
  fs.mkdirSync(resolvedBuildDir, { recursive: true });
  const snapshotAssetSource = path.join(resolvedSpecDir, 'snapshot-assets');
  if (fs.existsSync(snapshotAssetSource)) {
    fs.cpSync(
      snapshotAssetSource,
      path.join(resolvedBuildDir, 'snapshot-assets'),
      { recursive: true },
    );
  }

  const triggers = (spec.pages || [])
    .map((state) => triggerRecord(
      state,
      findTriggerNode(state, capture.nodes, capture.behaviors),
      spec.source?.capturedUrl || spec.source?.requestedUrl,
    ))
    .filter(Boolean);
  const sourceUrl = spec.source?.capturedUrl || spec.source?.requestedUrl;
  const responsiveByPath = {
    '/': responsiveStylesForState(spec.home, resolvedSpecDir),
  };
  for (const state of spec.pages || []) {
    const localPath = localPathForState(state, sourceUrl).split('?')[0];
    responsiveByPath[localPath] =
      responsiveStylesForState(state, resolvedSpecDir);
  }
  const { childrenOf, renderNode } = createRenderer(
    capture.nodes,
    triggers,
    sourceUrl,
  );
  const bodyNode = capture.nodes.find((node) => node.tag === 'body');
  if (!bodyNode) throw new Error('The specification has no body node.');

  const stylesheetSource = path.join(resolvedSpecDir, 'stylesheets');
  const stylesheetTarget = path.join(resolvedBuildDir, 'stylesheets');
  const cssFiles = fs.existsSync(stylesheetSource)
    ? fs.readdirSync(stylesheetSource).filter((file) => file.endsWith('.css')).sort()
    : [];
  fs.mkdirSync(stylesheetTarget, { recursive: true });
  for (const file of cssFiles) {
    fs.copyFileSync(
      path.join(stylesheetSource, file),
      path.join(stylesheetTarget, file),
    );
  }

  const cssLinks = cssFiles
    .map((file) => `<link rel="stylesheet" href="/stylesheets/${escapeAttribute(file)}">`)
    .join('\n');
  let homeHtml;
  const homeSourceFile = spec.home?.html
    ? path.join(resolvedSpecDir, spec.home.html)
    : '';
  if (homeSourceFile && fs.existsSync(homeSourceFile)) {
    let homeStylesheet = '';
    if (spec.home.stylesheet) {
      const stylesheetSourceFile = path.join(
        resolvedSpecDir,
        spec.home.stylesheet,
      );
      if (fs.existsSync(stylesheetSourceFile)) {
        const stateStylesDir = path.join(resolvedBuildDir, 'state-styles');
        fs.mkdirSync(stateStylesDir, { recursive: true });
        fs.copyFileSync(
          stylesheetSourceFile,
          path.join(stateStylesDir, 'home.css'),
        );
        homeStylesheet = '/state-styles/home.css';
      }
    }
    homeHtml = sanitizeCapturedHtml(
      fs.readFileSync(homeSourceFile, 'utf8'),
      spec.home.url || sourceUrl,
      cssFiles,
      homeStylesheet,
    );
  } else {
    const body = (childrenOf.get(bodyNode.path) || []).map(renderNode).join('\n');
    const title = escapeHtml(capture.document?.title || 'Site specification');
    homeHtml = `<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${title}</title>
  ${cssLinks}
  <script src="/recreate-runtime.js" defer></script>
</head>
<body>
${body}
</body>
</html>`;
  }
  fs.writeFileSync(path.join(resolvedBuildDir, 'index.html'), homeHtml);

  for (const state of spec.pages || []) {
    if (!state.html) continue;
    const sourceFile = path.join(resolvedSpecDir, state.html);
    if (!fs.existsSync(sourceFile)) continue;
    let stateStylesheet = '';
    if (state.stylesheet) {
      const stylesheetSourceFile = path.join(resolvedSpecDir, state.stylesheet);
      if (fs.existsSync(stylesheetSourceFile)) {
        const stylesheetName = `${String(state.index).padStart(3, '0')}.css`;
        const stateStylesDir = path.join(resolvedBuildDir, 'state-styles');
        fs.mkdirSync(stateStylesDir, { recursive: true });
        fs.copyFileSync(
          stylesheetSourceFile,
          path.join(stateStylesDir, stylesheetName),
        );
        stateStylesheet = `/state-styles/${stylesheetName}`;
      }
    }
    const targetPath = localPathForState(state, sourceUrl);
    const routePath = outputPathForState(state, targetPath);
    const targetDir = path.join(resolvedBuildDir, routePath);
    fs.mkdirSync(targetDir, { recursive: true });
    const stateHtml = sanitizeCapturedHtml(
      fs.readFileSync(sourceFile, 'utf8'),
      state.url,
      cssFiles,
      stateStylesheet,
    );
    fs.writeFileSync(path.join(targetDir, 'index.html'), stateHtml);
  }

  const manifest = {
    schemaVersion: 1,
    oracleOnly: true,
    deliveryPolicy:
      'Diagnostic reconstruction only. Never ship, embed, redirect to, or copy into a destination implementation.',
    source: sourceUrl,
    generatedAt: new Date().toISOString(),
    home: spec.home
      ? {
          sourceUrl: spec.home.url,
          stylesheet: spec.home.stylesheet,
        }
      : undefined,
    triggers,
    responsiveByPath,
    states: (spec.pages || []).map((state) => ({
      index: state.index,
      type: state.type,
      sourceUrl: state.url,
      localPath: localPathForState(state, sourceUrl),
      trigger: state.trigger,
      stylesheet: state.stylesheet,
    })),
  };
  fs.writeFileSync(
    path.join(resolvedBuildDir, 'recreate-manifest.json'),
    JSON.stringify(manifest, null, 2),
  );
  fs.writeFileSync(
    path.join(resolvedBuildDir, 'recreate-runtime.js'),
    runtimeSource(manifest),
  );
  fs.writeFileSync(
    path.join(resolvedBuildDir, 'ORACLE_ONLY.txt'),
    `${manifest.deliveryPolicy}\n`,
  );
  const queryRoutes = Object.fromEntries(
    (spec.pages || [])
      .map((state) => {
        const localPath = localPathForState(state, sourceUrl);
        return localPath.includes('?')
          ? [`${localPath}`, `/${outputPathForState(state, localPath)}`]
          : null;
      })
      .filter(Boolean),
  );
  fs.writeFileSync(
    path.join(resolvedBuildDir, 'server.mjs'),
    serverSource(queryRoutes),
  );

  return {
    buildDir: resolvedBuildDir,
    stateCount: manifest.states.length,
    triggerCount: triggers.length,
    stylesheetCount: cssFiles.length,
  };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = parseArgs(process.argv.slice(2));
  const specDir = String(args.spec || args['spec-dir'] || '');
  const buildDir = String(args.out || args['build-dir'] || '');
  if (!specDir || !buildDir) {
    throw new Error(
      'Usage: node build-static.mjs --spec <spec-directory> --out <build-directory>',
    );
  }
  const result = buildStatic({ specDir, buildDir });
  console.log(JSON.stringify(result, null, 2));
}
