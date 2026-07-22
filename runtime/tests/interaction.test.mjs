import assert from 'node:assert/strict';
import test from 'node:test';
import { closedInteraction, reduceInteraction } from '../interaction.mjs';

test('routes, switches, toggles, and restores the exact trigger', () => {
  const avatar = { type: 'activate', trigger: 8, surface: 2, stateful: true, closable: true };
  const search = { type: 'activate', trigger: 1, surface: 1, stateful: true, closable: true };
  const [account, openAccount] = reduceInteraction(closedInteraction, avatar);
  assert.deepEqual(openAccount, { type: 'open', surface: 2, trigger: 8 });
  const [searchState, openSearch] = reduceInteraction(account, search);
  assert.deepEqual(searchState, { openSurface: 1, activeTrigger: 1 });
  assert.deepEqual(openSearch, { type: 'open', surface: 1, trigger: 1 });
  const [closed, closeSearch] = reduceInteraction(searchState, search);
  assert.equal(closed, closedInteraction);
  assert.deepEqual(closeSearch, { type: 'close', restoreTrigger: 1 });
});

test('dismisses open surfaces and ignores dismissal while closed', () => {
  const open = { openSurface: 4, activeTrigger: 12 };
  for (const type of ['escape', 'outside']) {
    assert.deepEqual(reduceInteraction(open, { type }), [
      closedInteraction,
      { type: 'close', restoreTrigger: 12 },
    ]);
  }
  assert.deepEqual(reduceInteraction(closedInteraction, { type: 'escape' }), [
    closedInteraction,
    { type: 'none' },
  ]);
});

test('invokes non-closable behavior without opening a surface', () => {
  const event = { type: 'activate', trigger: 3, surface: 9, stateful: false, closable: false };
  assert.deepEqual(reduceInteraction(closedInteraction, event), [
    closedInteraction,
    { type: 'invoke', surface: 9, trigger: 3 },
  ]);
});

test('switches persistent state without making it dismissible', () => {
  const event = { type: 'activate', trigger: 3, surface: 9, stateful: true, closable: false };
  assert.deepEqual(reduceInteraction(closedInteraction, event), [
    { openSurface: 9, activeTrigger: 3 },
    { type: 'open', surface: 9, trigger: 3 },
  ]);
});
