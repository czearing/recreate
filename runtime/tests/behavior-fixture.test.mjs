import assert from 'node:assert/strict';
import test from 'node:test';
import { anchorParent } from '../anchor.mjs';
import { moveCarousel } from '../carousel.mjs';
import { closedInteraction, reduceInteraction } from '../interaction.mjs';
import { startSequence } from '../sequence.mjs';
import { FakeElement, fakeClock } from './support.mjs';

test('local behavior fixture wires routing, anchoring, copy, and carousel', () => {
  const search = new FakeElement();
  const avatar = new FakeElement();
  const cardA = new FakeElement();
  const cardB = new FakeElement();
  const searchParent = { id: 'search' };
  const accountParent = { id: 'account' };
  const cardAParent = { id: 'card-a' };
  const cardBParent = { id: 'card-b' };
  search.parentElement = searchParent;
  avatar.parentElement = accountParent;
  cardA.parentElement = cardAParent;
  cardB.parentElement = cardBParent;
  const root = {
    body: { id: 'body' },
    active: null,
    fallback: null,
    querySelector(selector) {
      return selector.includes('active') ? this.active : this.fallback;
    },
  };

  const [account] = reduceInteraction(closedInteraction, {
    type: 'activate',
    trigger: avatar,
    surface: 2,
    stateful: true,
    closable: true,
  });
  assert.equal(account.openSurface, 2);
  const [searchState] = reduceInteraction(account, {
    type: 'activate',
    trigger: search,
    surface: 1,
    stateful: true,
    closable: true,
  });
  assert.equal(searchState.activeTrigger, search);

  root.active = cardB;
  root.fallback = cardA;
  assert.equal(anchorParent(root, 4), cardBParent);

  const clock = fakeClock();
  const prompt = new FakeElement();
  const stop = startSequence(prompt, {
    attribute: 'textContent',
    steps: [
      { value: 'Launch plan', delay_ms: 4000 },
      { value: 'Browse recent items', delay_ms: 3500 },
    ],
  }, clock);
  clock.tick(4000);
  assert.equal(prompt.textContent, 'Browse recent items');
  stop();

  assert.equal(moveCarousel({ offset: 0, extent: 1096 }, 'forward').offset, 1096);
});
