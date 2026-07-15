import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { buildReactProject } from '../src/react-source/project.mjs';

test('emits readable React source without reconstruction runtime', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'recreate-react-'));
  const specDir = path.join(root, 'spec');
  const outDir = path.join(root, 'react');
  fs.mkdirSync(path.join(specDir, 'pages'), { recursive: true });
  fs.mkdirSync(path.join(specDir, 'stylesheets'), { recursive: true });
  fs.writeFileSync(path.join(specDir, 'spec.json'), JSON.stringify({
    home: { html: 'pages/home.html', stylesheet: 'pages/home.css', title: 'Fixture' },
  }));
  fs.writeFileSync(path.join(specDir, 'pages', 'home.html'),
    '<html><body><main><h1 class="hero">Hello</h1><button class="action" disabled>Go' +
    '<svg width="16" height="16" fill="currentColor"><path d="M0 0h1v1z" fill="currentColor"/></svg>' +
    '</button></main></body></html>');
  fs.writeFileSync(path.join(specDir, 'pages', 'home.css'),
    '.hero{color:red}.action{color:#123456}.action svg{width:16px}.unused{display:none}');
  fs.writeFileSync(path.join(specDir, 'stylesheets', '0000.css'),
    'body{margin:0}.hero{color:red}');

  const result = buildReactProject({ specDir, outDir, maxNodes: 4 });
  const source = fs.readdirSync(path.join(outDir, 'src', 'components'))
    .filter((file) => file.endsWith('.tsx'))
    .map((file) => fs.readFileSync(path.join(outDir, 'src', 'components', file), 'utf8'))
    .join('\n');
  assert.ok(result.componentCount >= 1);
  assert.ok(result.maxComponentLines < 200);
  assert.match(source, /<h1 className="hero">/);
  assert.match(source, /disabled/);
  assert.doesNotMatch(source, /dangerouslySetInnerHTML|recreate-runtime|application\/json/);
  assert.doesNotMatch(source, /<svg/);
  const assetFiles = fs.readdirSync(path.join(outDir, 'public', 'assets'));
  assert.equal(assetFiles.length, 1);
  assert.doesNotMatch(
    fs.readFileSync(path.join(outDir, 'public', 'assets', assetFiles[0]), 'utf8'),
    /currentColor/,
  );
  const css = [
    fs.readFileSync(path.join(outDir, 'src', 'styles', 'shared.css'), 'utf8'),
    ...fs.readdirSync(path.join(outDir, 'src', 'components'))
      .filter((file) => file.endsWith('.css'))
      .map((file) => fs.readFileSync(path.join(outDir, 'src', 'components', file), 'utf8')),
  ].join('\n');
  assert.match(css, /body \{\n\s+margin:0/);
  assert.doesNotMatch(css, /\.unused/);
  assert.equal((css.match(/\.hero/g) || []).length, 1);
  assert.match(css, /\.action img/);
});

test('deduplicates repeated structures into one typed component', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'recreate-react-repeat-'));
  const specDir = path.join(root, 'spec');
  const outDir = path.join(root, 'react');
  fs.mkdirSync(path.join(specDir, 'pages'), { recursive: true });
  fs.mkdirSync(path.join(specDir, 'stylesheets'), { recursive: true });
  fs.writeFileSync(path.join(specDir, 'spec.json'), JSON.stringify({
    home: { html: 'pages/home.html', title: 'Repeated fixture' },
  }));
  fs.writeFileSync(path.join(specDir, 'pages', 'home.html'), [
    '<html><body><main><div>',
    '<article><h2>First</h2><p>Alpha</p></article>',
    '<article><h2>Second</h2><p>Beta</p></article>',
    '</div></main></body></html>',
  ].join(''));

  buildReactProject({ specDir, outDir });
  const sources = fs.readdirSync(path.join(outDir, 'src', 'components'))
    .filter((file) => file.endsWith('.tsx'))
    .map((file) => fs.readFileSync(path.join(outDir, 'src', 'components', file), 'utf8'));
  const item = sources.find((source) => /interface \w+ItemProps/.test(source));
  const parent = sources.find((source) => /text="First" text2="Alpha"/.test(source));
  assert.match(item, /\{text\}/);
  assert.match(parent, /text="Second" text2="Beta"/);
});
