import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { buildReactProject } from '../src/react-source/project.mjs';

test('emits readable React source without reconstruction runtime', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'site-spec-react-'));
  const specDir = path.join(root, 'spec');
  const outDir = path.join(root, 'react');
  fs.mkdirSync(path.join(specDir, 'pages'), { recursive: true });
  fs.mkdirSync(path.join(specDir, 'stylesheets'), { recursive: true });
  fs.writeFileSync(path.join(specDir, 'spec.json'), JSON.stringify({
    home: { html: 'pages/home.html', stylesheet: 'pages/home.css', title: 'Fixture' },
  }));
  fs.writeFileSync(path.join(specDir, 'pages', 'home.html'),
    '<html><body><main><h1>Hello</h1><button disabled>Go</button></main></body></html>');
  fs.writeFileSync(path.join(specDir, 'pages', 'home.css'), 'h1{color:red}');
  fs.writeFileSync(path.join(specDir, 'stylesheets', '0000.css'), 'body{margin:0}');

  const result = buildReactProject({ specDir, outDir, maxNodes: 4 });
  const source = fs.readdirSync(path.join(outDir, 'src', 'components'))
    .map((file) => fs.readFileSync(path.join(outDir, 'src', 'components', file), 'utf8'))
    .join('\n');
  assert.ok(result.componentCount >= 1);
  assert.ok(result.maxComponentLines < 200);
  assert.match(source, /<h1>/);
  assert.match(source, /disabled/);
  assert.doesNotMatch(source, /dangerouslySetInnerHTML|site-spec-runtime|application\/json/);
  assert.match(fs.readFileSync(path.join(outDir, 'src', 'styles', '00-0000.css'), 'utf8'),
    /body \{\n  margin:0/);
});

test('deduplicates repeated structures into one typed component', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'site-spec-react-repeat-'));
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
    .map((file) => fs.readFileSync(path.join(outDir, 'src', 'components', file), 'utf8'));
  const item = sources.find((source) => /interface \w+ItemProps/.test(source));
  const parent = sources.find((source) => /text="First" text2="Alpha"/.test(source));
  assert.match(item, /\{text\}/);
  assert.match(parent, /text="Second" text2="Beta"/);
});
