import assert from 'node:assert/strict';
import test from 'node:test';
import {
  candidateUsesTextEntry,
  interactionCandidatePriority,
  interactionMatchPriority,
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
