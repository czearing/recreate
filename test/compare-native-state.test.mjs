import assert from 'node:assert/strict';
import test from 'node:test';

import { compareNativeState } from '../src/compare-native-state.mjs';

const node = (label, x, opacity = '1') => ({
  tag: 'button',
  text: label,
  rect: { x, y: 0, width: 10, height: 10 },
  style: { display: 'block', opacity },
});

test('matches duplicate identities by nearest geometry', () => {
  const result = compareNativeState(
    { nodes: [node('More', 10), node('More', 100)] },
    { nodes: [node('More', 98), node('More', 12)] },
  );
  assert.equal(result.matched, 2);
  assert.equal(result.maxDeltaPx, 2);
});

test('reports hidden controls separately from painted geometry', () => {
  const result = compareNativeState(
    { nodes: [node('Create', 10, '0'), node('Open', 20)] },
    { nodes: [node('Create', 80, '0'), node('Open', 21)] },
  );
  assert.equal(result.maxDeltaPx, 70);
  assert.equal(result.painted.maxDeltaPx, 1);
});

test('ignores clipped controls that are not operable', () => {
  const clipped = node('Create', 10);
  clipped.rect.width = 2;
  const result = compareNativeState({ nodes: [clipped] }, { nodes: [] });
  assert.equal(result.required, 0);
  assert.equal(result.missing.length, 0);
});

test('matches visible leaf text across different native tags', () => {
  const result = compareNativeState(
    {
      nodes: [{
        tag: 'div',
        path: 'reference-title',
        text: 'From ideas to progress',
        rect: { x: 10, y: 20, width: 100, height: 40 },
        style: { display: 'block', opacity: '1' },
      }],
    },
    {
      nodes: [{
        tag: 'h1',
        path: 'candidate-title',
        text: 'From ideas to progress',
        rect: { x: 12, y: 20, width: 100, height: 40 },
        style: { display: 'block', opacity: '1' },
      }],
    },
  );
  assert.equal(result.required, 1);
  assert.equal(result.matched, 1);
  assert.equal(result.maxDeltaPx, 2);
});

test('does not count wrapper innerText when a child owns the text', () => {
  const state = {
    nodes: [
      {
        tag: 'div',
        path: 'wrapper',
        text: 'Hello',
        rect: { x: 0, y: 0, width: 100, height: 20 },
        style: { display: 'block', opacity: '1' },
      },
      {
        tag: 'span',
        path: 'leaf',
        parentPath: 'wrapper',
        text: 'Hello',
        rect: { x: 0, y: 0, width: 40, height: 20 },
        style: { display: 'block', opacity: '1' },
      },
    ],
  };
  const result = compareNativeState(state, state);
  assert.equal(result.required, 1);
});

test('does not double count text owned by an interactive parent', () => {
  const state = {
    nodes: [
      {
        tag: 'button',
        path: 'button',
        text: 'Notebooks',
        rect: { x: 0, y: 0, width: 100, height: 32 },
        style: { display: 'block', opacity: '1' },
      },
      {
        tag: 'span',
        path: 'label',
        parentPath: 'button',
        text: 'Notebooks',
        rect: { x: 10, y: 6, width: 80, height: 20 },
        style: { display: 'block', opacity: '1' },
      },
    ],
  };
  const result = compareNativeState(state, state);
  assert.equal(result.required, 1);
  assert.equal(result.missing.length, 0);
});

test('compares semantic and visible text identities on one leaf element', () => {
  const state = {
    nodes: [{
      tag: 'span',
      path: 'avatar',
      ariaLabel: 'Ed Maurer',
      text: 'EM',
      rect: { x: 0, y: 0, width: 28, height: 28 },
      style: { display: 'block', opacity: '1' },
    }],
  };
  const result = compareNativeState(state, state);
  assert.equal(result.required, 2);
  assert.equal(result.matched, 2);
});

test('ignores paint on transparent semantic wrappers with child content', () => {
  const reference = {
    nodes: [
      {
        tag: 'span',
        path: 'avatar',
        ariaLabel: 'Ed Maurer',
        text: 'Ed Maurer',
        rect: { x: 0, y: 0, width: 28, height: 28 },
        style: {
          display: 'block',
          opacity: '1',
          backgroundColor: 'rgba(0, 0, 0, 0)',
          border: '0px none rgb(0, 0, 0)',
          boxShadow: 'none',
        },
      },
      {
        tag: 'span',
        path: 'initials',
        parentPath: 'avatar',
        text: 'EM',
        rect: { x: 0, y: 0, width: 28, height: 28 },
        style: { display: 'block', opacity: '1' },
      },
    ],
  };
  const candidate = structuredClone(reference);
  candidate.nodes[0].style.backgroundColor = 'rgb(255, 255, 255)';
  candidate.nodes[0].style.border = '2px solid rgb(0, 0, 0)';
  const result = compareNativeState(reference, candidate);
  assert.equal(result.paint.mismatched, 0);
});

test('reports exact paint property differences', () => {
  const reference = node('Open', 10);
  reference.style.backgroundColor = 'rgb(255, 255, 255)';
  const candidate = node('Open', 10);
  candidate.style.backgroundColor = 'rgb(245, 245, 245)';
  const result = compareNativeState({ nodes: [reference] }, { nodes: [candidate] });
  assert.equal(result.paint.mismatched, 1);
  assert.deepEqual(result.paint.properties, { backgroundColor: 1 });
});

test('treats equivalent subpixel and one-pixel borders as thin borders', () => {
  const reference = node('Open', 10);
  reference.style.border = '0.666667px solid rgb(224, 224, 224)';
  const candidate = node('Open', 10);
  candidate.style.border = '1px solid rgb(224, 224, 224)';
  const result = compareNativeState({ nodes: [reference] }, { nodes: [candidate] });
  assert.equal(result.paint.mismatched, 0);
});

test('leaves the distant duplicate unmatched instead of shifting every pair', () => {
  const result = compareNativeState(
    { nodes: [node('Same', 0), node('Same', 100), node('Same', 200)] },
    { nodes: [node('Same', 100), node('Same', 200)] },
  );
  assert.equal(result.matched, 2);
  assert.equal(result.missing.length, 1);
  assert.equal(result.maxDeltaPx, 0);
});
