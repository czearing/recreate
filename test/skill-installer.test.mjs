import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  installSkill,
  selectInstallTargets,
} from '../src/skill-installer.mjs';
import {
  currentSkillInstructions,
  installedSkillContent,
} from '../src/skill-content.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

function withTemporaryHome(callback) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'recreate-skill-'));
  try {
    return callback(directory);
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
}

test('builds a shared skill that resolves the latest npm release', () => {
  const content = installedSkillContent();
  assert.match(content, /^---\nname: recreate\n/);
  assert.match(
    content,
    /--registry=https:\/\/registry\.npmjs\.org\/ recreate-cli@latest skill/,
  );
  const instructions = currentSkillInstructions();
  assert.match(instructions, /acceptance-matrix\.json/);
  assert.match(instructions, /page as a whole/);
  assert.match(instructions, /access page itself may be the requested interface/);
  assert.match(instructions, /--reuse --target <target-id>/);
  assert.match(instructions, /Chrome, Edge, or Chromium/);
  assert.match(instructions, /Do not ask the user to start/);
  assert.match(instructions, /Probe http:\/\/127\.0\.0\.1:9222\/json\/version/);
  assert.match(instructions, /Open the source URL.*through CDP/);
  assert.match(instructions, /only browser setup question permitted/);
  assert.match(instructions, /Do not substitute HTTP fetches/);
  assert.match(instructions, /without internal option names/);
  assert.doesNotMatch(instructions, /regex|text matching/i);
});

test('detects existing Copilot and Claude personal homes', () =>
  withTemporaryHome((home) => {
    fs.mkdirSync(path.join(home, '.copilot'));
    fs.mkdirSync(path.join(home, '.claude'));
    assert.deepEqual(selectInstallTargets([], home, { PATH: '' }), [
      'copilot',
      'claude',
    ]);
  }));

test('supports explicit client selection', () =>
  withTemporaryHome((home) => {
    assert.deepEqual(selectInstallTargets(['--copilot'], home), ['copilot']);
    assert.deepEqual(selectInstallTargets(['--claude'], home), ['claude']);
    assert.deepEqual(selectInstallTargets(['--all'], home), [
      'copilot',
      'claude',
    ]);
    assert.throws(
      () => selectInstallTargets(['--unknown'], home),
      /Unknown install option/,
    );
  }));

test('installs both personal skills idempotently', () =>
  withTemporaryHome((home) => {
    installSkill(['--all'], home);
    installSkill(['--all'], home);
    for (const client of ['.copilot', '.claude']) {
      const content = fs.readFileSync(
        path.join(home, client, 'skills', 'recreate', 'SKILL.md'),
        'utf8',
      );
      assert.equal(content, installedSkillContent());
    }
  }));

test('does not overwrite a linked skill directory', () =>
  withTemporaryHome((home) => {
    const target = path.join(home, 'shared-skill');
    const link = path.join(home, '.claude', 'skills', 'recreate');
    fs.mkdirSync(target, { recursive: true });
    fs.mkdirSync(path.dirname(link), { recursive: true });
    fs.symlinkSync(target, link, 'junction');
    assert.throws(() => installSkill(['--claude'], home), /linked skill/);
  }));

test('runs the terminal installer against an isolated home', () =>
  withTemporaryHome((home) => {
    execFileSync(
      process.execPath,
      [path.join(root, 'src', 'cli.mjs'), 'install', '--all'],
      {
        encoding: 'utf8',
        env: { ...process.env, RECREATE_HOME: home },
      },
    );
    assert.ok(
      fs.existsSync(
        path.join(home, '.copilot', 'skills', 'recreate', 'SKILL.md'),
      ),
    );
    assert.ok(
      fs.existsSync(
        path.join(home, '.claude', 'skills', 'recreate', 'SKILL.md'),
      ),
    );
  }));
