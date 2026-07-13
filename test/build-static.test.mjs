import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { buildStatic } from '../build-static.mjs';

const htmlPath = 'doc(0)>html:nth-of-type(1)';
const bodyPath = `${htmlPath}>body:nth-of-type(1)`;
const rootPath = `${bodyPath}>div:nth-of-type(1)`;

function node(pathname, parentPath, tag, attrs = {}, text = '') {
  return {
    path: pathname,
    parentPath,
    tag,
    attrs,
    text,
    visible: true,
    style: { display: 'block' },
  };
}

test('builds local state routes and wires distinct interactive controls', () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'site-spec-build-'));
  const specDir = path.join(temp, 'spec');
  const buildDir = path.join(temp, 'build');
  fs.mkdirSync(path.join(specDir, 'pages'), { recursive: true });
  fs.mkdirSync(path.join(specDir, 'stylesheets'), { recursive: true });
  fs.mkdirSync(path.join(specDir, 'evidence'), { recursive: true });
  fs.writeFileSync(path.join(specDir, 'stylesheets', '0000.css'), 'body{color:#111}');

  const selectorPath = `${rootPath}>button:nth-of-type(1)`;
  const cardPath = `${rootPath}>div:nth-of-type(1)`;
  const taskPath = `${rootPath}>button:nth-of-type(2)`;
  const queryPath = `${rootPath}>button:nth-of-type(3)`;
  const nodes = [
    node(htmlPath, 'doc(0)', 'html'),
    node(bodyPath, htmlPath, 'body'),
    node(rootPath, bodyPath, 'div', { id: 'root' }),
    node(selectorPath, rootPath, 'button', {
      'aria-haspopup': 'listbox',
      title: 'Asking on: My Notebook',
    }, 'My Notebook'),
    node(cardPath, rootPath, 'div', {
      role: 'button',
      'aria-label': 'My Notebook',
      'data-testid': 'notebook-card',
    }),
    node(taskPath, rootPath, 'button', {
      'aria-label': 'Build a status deck. Adds to My Notebook.',
    }),
    node(queryPath, rootPath, 'button', {
      'aria-label': 'Open compact notebook view',
    }),
  ];

  for (let index = 0; index < 4; index++) {
    fs.writeFileSync(
      path.join(specDir, 'pages', `${String(index).padStart(3, '0')}.html`),
      `<!doctype html><html><head><base href="/app/"><link rel="stylesheet" crossorigin href="./app.css"><script src="app.js"></script></head><body><button title="Home">Home</button><p>state-${index}</p></body></html>`,
    );
  }
  fs.writeFileSync(path.join(specDir, 'pages', '000.css'), '.route{display:block}');

  fs.writeFileSync(
    path.join(specDir, 'evidence', 'capture-1440x900.json'),
    JSON.stringify({
      document: { title: 'Fixture' },
      nodes,
      behaviors: [],
    }),
  );
  fs.writeFileSync(
    path.join(specDir, 'spec.json'),
    JSON.stringify({
      source: { capturedUrl: 'https://example.test/app/' },
      captures: [{ file: 'evidence/capture-1440x900.json' }],
      pages: [
        {
          index: 0,
          type: 'route',
          url: 'https://example.test/app/notebook/my-notebook',
          html: 'pages/000.html',
          stylesheet: 'pages/000.css',
          text: 'My Notebook',
        },
        {
          index: 1,
          type: 'panel',
          trigger: 'My Notebook',
          url: 'https://example.test/app/',
          html: 'pages/001.html',
        },
        {
          index: 2,
          type: 'route',
          url: 'https://example.test/chat/session-123',
          html: 'pages/002.html',
          text: 'Here are the deliverables for "Build a status deck".',
        },
        {
          index: 3,
          type: 'route',
          url: 'https://example.test/app/notebook/my-notebook?view=compact',
          html: 'pages/003.html',
          trigger: 'Open compact notebook view',
          triggerElement: {
            path: queryPath,
            label: 'Open compact notebook view',
            tag: 'button',
            role: '',
            testId: '',
          },
        },
      ],
    }),
  );

  const result = buildStatic({ specDir, buildDir });
  assert.equal(result.stateCount, 4);
  assert.equal(result.triggerCount, 4);

  const home = fs.readFileSync(path.join(buildDir, 'index.html'), 'utf8');
  assert.match(
    home,
    /data-site-spec-target="\/app\/notebook\/my-notebook"/,
  );
  assert.match(
    home,
    /data-site-spec-target="\/__site-spec\/state\/001"/,
  );
  assert.match(home, /data-site-spec-target="\/chat\/session-123"/);
  assert.match(
    home,
    /data-site-spec-target="\/app\/notebook\/my-notebook\?view=compact"/,
  );

  const route = fs.readFileSync(
    path.join(buildDir, 'app', 'notebook', 'my-notebook', 'index.html'),
    'utf8',
  );
  assert.doesNotMatch(route, /<script src="app\.js"/);
  assert.doesNotMatch(route, /crossorigin/i);
  assert.doesNotMatch(route, /href="\.\/app\.css"/);
  assert.match(route, /data-site-spec-href="\/stylesheets\/0000\.css"/);
  assert.match(route, /data-site-spec-href="\/state-styles\/000\.css"/);
  assert.match(route, /<base href="https:\/\/example\.test\/app\/">/);
  assert.match(route, /script\.src=location\.origin\+"\/site-spec-runtime\.js"/);

  const manifest = JSON.parse(
    fs.readFileSync(path.join(buildDir, 'site-spec-manifest.json'), 'utf8'),
  );
  const routeTrigger = manifest.triggers.find(
    (trigger) => trigger.target === '/app/notebook/my-notebook',
  );
  assert.equal(routeTrigger.testId, 'notebook-card');
  assert.equal(routeTrigger.tag, 'div');
  assert.equal(
    fs.existsSync(path.join(buildDir, '__site-spec', 'state', '001', 'index.html')),
    true,
  );
  assert.equal(
    fs.readFileSync(path.join(buildDir, 'state-styles', '000.css'), 'utf8'),
    '.route{display:block}',
  );
  assert.match(route, /state-0/);
  const queryRoute = fs.readFileSync(
    path.join(buildDir, '__site-spec', 'query', '003', 'index.html'),
    'utf8',
  );
  assert.match(queryRoute, /state-3/);
  const runtime = fs.readFileSync(
    path.join(buildDir, 'site-spec-runtime.js'),
    'utf8',
  );
  assert.match(runtime, /new URL\(target, location\.origin\)/);
  assert.match(runtime, /history\.pushState\(null, '', location\.href\)/);
  assert.match(runtime, /location\.assign\(localUrl\.href\)/);
  assert.match(runtime, /document\.querySelector\(selector\)/);
  assert.match(runtime, /if \(!expected\) return false/);
  assert.match(runtime, /actual\.endsWith\(' ' \+ expected\)/);
  const server = fs.readFileSync(path.join(buildDir, 'server.mjs'), 'utf8');
  assert.match(
    server,
    /"\/app\/notebook\/my-notebook\?view=compact":"\/__site-spec\/query\/003"/,
  );
});

test('emits viewport-specific layout overrides from state evidence', () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'site-spec-responsive-'));
  const specDir = path.join(temp, 'spec');
  const buildDir = path.join(temp, 'build');
  fs.mkdirSync(path.join(specDir, 'evidence'), { recursive: true });
  fs.mkdirSync(path.join(specDir, 'pages'), { recursive: true });
  fs.writeFileSync(
    path.join(specDir, 'pages', 'home.html'),
    '<!doctype html><html><head></head><body><div id="root"></div></body></html>',
  );
  const pathValue = 'doc(0)>html:nth-of-type(1)>body:nth-of-type(1)>div:nth-of-type(1)';
  fs.writeFileSync(
    path.join(specDir, 'evidence', 'capture.json'),
    JSON.stringify({
      document: { title: 'Responsive' },
      behaviors: [],
      nodes: [
        node(htmlPath, 'doc(0)', 'html'),
        node(bodyPath, htmlPath, 'body'),
        node(pathValue, bodyPath, 'div', { id: 'root' }),
      ],
    }),
  );
  fs.writeFileSync(
    path.join(specDir, 'evidence', 'home-desktop.json'),
    JSON.stringify({
      viewport: { width: 1440, height: 900 },
      nodes: [{ path: pathValue, style: { display: 'flex', flexDirection: 'row' } }],
    }),
  );
  fs.writeFileSync(
    path.join(specDir, 'evidence', 'home-mobile.json'),
    JSON.stringify({
      viewport: { width: 390, height: 844 },
      nodes: [{ path: pathValue, style: { display: 'flex', flexDirection: 'column' } }],
    }),
  );
  fs.writeFileSync(
    path.join(specDir, 'spec.json'),
    JSON.stringify({
      source: { capturedUrl: 'https://example.test/' },
      captures: [{ file: 'evidence/capture.json' }],
      home: {
        url: 'https://example.test/',
        html: 'pages/home.html',
        evidence: 'evidence/home-desktop.json',
        evidenceByViewport: {
          '1440x900': 'evidence/home-desktop.json',
          '390x844': 'evidence/home-mobile.json',
        },
      },
      pages: [],
    }),
  );

  buildStatic({ specDir, buildDir });
  const manifest = JSON.parse(
    fs.readFileSync(path.join(buildDir, 'site-spec-manifest.json'), 'utf8'),
  );
  assert.deepEqual(manifest.responsiveByPath['/'], [{
    maxWidth: 390,
    entries: [{
      path: pathValue,
      style: { 'flex-direction': 'column' },
    }],
  }]);
  const runtime = fs.readFileSync(
    path.join(buildDir, 'site-spec-runtime.js'),
    'utf8',
  );
  assert.match(runtime, /style\.setProperty\(property, value, 'important'\)/);
});

test('uses the settled captured home document when available', () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'site-spec-home-'));
  const specDir = path.join(temp, 'spec');
  const buildDir = path.join(temp, 'build');
  fs.mkdirSync(path.join(specDir, 'pages'), { recursive: true });
  fs.mkdirSync(path.join(specDir, 'snapshot-assets'), { recursive: true });
  fs.writeFileSync(
    path.join(specDir, 'snapshot-assets', 'fixture.png'),
    Buffer.from([137, 80, 78, 71]),
  );
  fs.writeFileSync(
    path.join(specDir, 'pages', 'home.html'),
    '<!doctype html><html><head><script src="app.js"></script></head><body><main data-captured-home>Settled home<img src="/snapshot-assets/fixture.png"><img src="blob:https://example.test/avatar"></main></body></html>',
  );
  fs.writeFileSync(
    path.join(specDir, 'pages', 'home.css'),
    'main{display:block}',
  );
  fs.writeFileSync(
    path.join(specDir, 'spec.json'),
    JSON.stringify({
      source: { capturedUrl: 'https://example.test/app/' },
      home: {
        type: 'home',
        url: 'https://example.test/app/',
        html: 'pages/home.html',
        stylesheet: 'pages/home.css',
      },
      captures: [{
        document: { title: 'Fixture' },
        nodes: [
          node(htmlPath, 'doc(0)', 'html'),
          node(bodyPath, htmlPath, 'body'),
          node(rootPath, bodyPath, 'div', { id: 'loading-fallback' }),
        ],
      }],
      pages: [],
    }),
  );

  buildStatic({ specDir, buildDir });

  const home = fs.readFileSync(path.join(buildDir, 'index.html'), 'utf8');
  assert.match(home, /data-captured-home/);
  assert.match(home, /data-site-spec-href="\/state-styles\/home\.css"/);
  assert.doesNotMatch(home, /loading-fallback/);
  assert.doesNotMatch(home, /<script src="app\.js"/);
  assert.doesNotMatch(home, /blob:https:/);
  assert.match(home, /data:image\/gif;base64/);
  assert.equal(
    fs.existsSync(path.join(buildDir, 'snapshot-assets', 'fixture.png')),
    true,
  );
  assert.equal(
    fs.readFileSync(path.join(buildDir, 'state-styles', 'home.css'), 'utf8'),
    'main{display:block}',
  );
});
