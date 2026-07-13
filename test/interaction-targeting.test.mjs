import assert from 'node:assert/strict';
import test from 'node:test';
import {
  candidateUsesTextEntry,
  interactionMatchPriority,
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
