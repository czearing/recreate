import assert from 'node:assert/strict';
import test from 'node:test';
import { moveCarousel } from '../carousel.mjs';

test('moves to the measured extent and returns to origin', () => {
  const origin = { offset: 0, extent: 1096, previousDisabled: true, nextDisabled: false };
  const advanced = moveCarousel(origin, 'forward');
  assert.deepEqual(advanced, {
    offset: 1096,
    extent: 1096,
    previousDisabled: false,
    nextDisabled: true,
  });
  assert.deepEqual(moveCarousel(advanced, 'backward'), origin);
});

test('clamps invalid extents to zero', () => {
  assert.deepEqual(
    moveCarousel({ offset: 10, extent: -2 }, 'forward'),
    { offset: 0, extent: -2, previousDisabled: true, nextDisabled: true },
  );
});
