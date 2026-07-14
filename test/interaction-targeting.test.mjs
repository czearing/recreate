import assert from 'node:assert/strict';
import test from 'node:test';
import {
  candidateUsesTextEntry,
  interactionCandidatePriority,
  interactionIdentity,
  interactionMatchPriority,
  interactionTargetPriority,
  interactionSettleTimeout,
  interactionStateSettleDelay,
  selectInteractionIdentity,
} from '../src/interaction-targeting.mjs';

test('matches exact interaction labels before partial labels', () => {
  assert.equal(interactionMatchPriority({
    text: '',
    placeholder: 'Select an animal',
  }, 'select an animal'), 2);
  assert.equal(interactionMatchPriority({
    text: 'Open animal picker',
  }, 'animal'), 1);
});

test('normalizes repeated controls to one component interaction identity', () => {
  assert.equal(
    interactionIdentity({ label: ' More   options ', tag: 'BUTTON' }),
    interactionIdentity({ text: 'More options', tag: 'button' }),
  );
});

test('targets duplicate controls by exact captured path', () => {
  assert.equal(interactionTargetPriority({
    text: 'More options',
    snapshotPath: 'doc(0)>button:nth-of-type(2)',
  }, [{
    path: 'doc(0)>button:nth-of-type(2)',
    label: 'More options',
  }]), 3);
  assert.equal(interactionTargetPriority({
    text: 'More options',
    snapshotPath: 'doc(0)>button:nth-of-type(1)',
  }, [{
    path: 'doc(0)>button:nth-of-type(2)',
    label: 'More options',
  }, {
    path: 'doc(0)>button:nth-of-type(3)',
    label: 'More options',
  }]), 0);
});

test('targets duplicate controls by captured geometry when paths drift', () => {
  assert.equal(interactionTargetPriority({
    text: 'Create',
    snapshotPath: 'new-path',
    rect: { x: 100, y: 200, width: 56, height: 26 },
  }, [{
    path: 'old-path-1',
    label: 'Create',
    rect: { x: 20, y: 200, width: 56, height: 26 },
  }, {
    path: 'old-path-2',
    label: 'Create',
    rect: { x: 100, y: 200, width: 56, height: 26 },
  }]), 3);
});

test('does not mistake card focus state for completed navigation', () => {
  assert.equal(
    interactionStateSettleDelay({ testId: 'notebook-card' }),
    5000,
  );
  assert.equal(interactionStateSettleDelay({ tag: 'BUTTON' }, true), 1200);
});

test('allows data-backed navigation cards more time to settle', () => {
  assert.equal(interactionSettleTimeout({ testId: 'notebook-card' }), 8000);
  assert.equal(interactionSettleTimeout({ text: "Build a deck. Adds to Notes. You'll get Deck." }), 25000);
  assert.equal(interactionSettleTimeout({ tag: 'BUTTON' }), 3000);
});

test('probes navigation cards before side-effecting free text', () => {
  assert.ok(
    interactionCandidatePriority({ testId: 'notebook-card' }) >
    interactionCandidatePriority({ tag: 'TEXTAREA' }),
  );
});

test('rejects ambiguous shared label prefixes after preferring exact identity', () => {
  const labels = [
    'Edit icon for Project Horizon - M&A Integration',
    'Edit icon for Project Starline MVP Launch',
  ];
  assert.equal(
    selectInteractionIdentity('Edit icon for Project Starline MVP Launch', labels),
    1,
  );
  assert.equal(selectInteractionIdentity('Edit icon for Project', labels), -1);
});

test('activates combobox inputs instead of mutating their text', () => {
  assert.equal(candidateUsesTextEntry({
    tag: 'INPUT',
    inputType: 'text',
    role: 'combobox',
  }), false);
  assert.equal(candidateUsesTextEntry({
    tag: 'INPUT',
    inputType: 'text',
  }), true);
});
