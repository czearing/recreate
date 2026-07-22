import assert from 'node:assert/strict';
import test from 'node:test';
import { applySequenceValue, startSequence } from '../sequence.mjs';
import { FakeElement, fakeClock } from './support.mjs';

const sequence = {
  attribute: 'textContent',
  steps: [
    { value: 'Launch plan', delay_ms: 4000 },
    { value: 'Competitive analysis', delay_ms: 3500 },
    { value: 'Browse recent items', delay_ms: 4250 },
    { value: 'Draft suggestions', delay_ms: 3000 },
  ],
};

test('plays irregular steps and loops twice without wall-clock waits', () => {
  const clock = fakeClock();
  const element = new FakeElement();
  const stop = startSequence(element, sequence, clock);
  assert.equal(element.textContent, 'Launch plan');
  for (const [delay, value] of [
    [4000, 'Competitive analysis'],
    [3500, 'Browse recent items'],
    [4250, 'Draft suggestions'],
    [3000, 'Launch plan'],
    [4000, 'Competitive analysis'],
    [3500, 'Browse recent items'],
    [4250, 'Draft suggestions'],
    [3000, 'Launch plan'],
  ]) {
    clock.tick(delay);
    assert.equal(element.textContent, value);
  }
  stop();
  assert.equal(clock.pending(), 0);
});

test('applies attributes separately from text content', () => {
  const element = new FakeElement();
  applySequenceValue(element, { attribute: 'placeholder' }, 'Ask anything');
  assert.equal(element.attributes.get('placeholder'), 'Ask anything');
  applySequenceValue(element, { attribute: 'textContent' }, 'Visible copy');
  assert.equal(element.textContent, 'Visible copy');
});

test('resumes from the captured mid-cycle value without changing layout state', () => {
  const clock = fakeClock();
  const element = new FakeElement();
  element.textContent = 'Browse recent items';
  const stop = startSequence(element, sequence, clock);
  assert.equal(element.textContent, 'Browse recent items');
  clock.tick(4250);
  assert.equal(element.textContent, 'Draft suggestions');
  stop();
});

test('preserves captured direct text nodes and embedded elements', () => {
  const clock = fakeClock();
  const first = { nodeType: 3, nodeValue: 'Browse recent items' };
  const spacer = { nodeType: 3, nodeValue: ' for ' };
  const child = { nodeType: 1, textContent: '' };
  const suffix = { nodeType: 3, nodeValue: 'project notes.' };
  const element = {
    childNodes: [first, spacer, child, suffix],
    get textContent() {
      return this.childNodes
        .map(node => node.nodeType === 3 ? node.nodeValue : node.textContent)
        .join('');
    },
  };
  const structuredSequence = {
    ...sequence,
    steps: [
      { value: 'Browse recent items for project notes.', delay_ms: 4250 },
      { value: 'Draft suggestions', delay_ms: 3000 },
    ],
  };

  const stop = startSequence(element, structuredSequence, clock);
  assert.deepEqual(element.childNodes, [first, spacer, child, suffix]);
  assert.equal(spacer.nodeValue, ' for ');
  assert.equal(suffix.nodeValue, 'project notes.');

  clock.tick(4250);
  assert.deepEqual(element.childNodes, [first, spacer, child, suffix]);
  assert.equal(first.nodeValue + spacer.nodeValue + suffix.nodeValue, 'Draft suggestions');
  assert.ok(first.nodeValue);
  assert.ok(spacer.nodeValue);
  assert.ok(suffix.nodeValue);
  stop();
});

test('retains captured static text segments while rotating the dynamic segment', () => {
  const first = { nodeType: 3, nodeValue: 'Make a project' };
  const spacer = { nodeType: 3, nodeValue: ' for ' };
  const suffix = { nodeType: 3, nodeValue: 'project notes.' };
  const child = { nodeType: 1, textContent: '' };
  const element = { childNodes: [first, spacer, suffix, child] };

  applySequenceValue(element, sequence, 'Make a project for Q4 planning.');

  assert.equal(first.nodeValue, 'Make a project');
  assert.equal(spacer.nodeValue, ' for ');
  assert.equal(suffix.nodeValue, 'Q4 planning.');
  assert.deepEqual(element.childNodes, [first, spacer, suffix, child]);
});

test('does not schedule incomplete sequences', () => {
  const clock = fakeClock();
  const stop = startSequence(new FakeElement(), { attribute: 'title', steps: [] }, clock);
  stop();
  assert.equal(clock.pending(), 0);
});

test('keeps captured sequence state frozen during exact verification', () => {
  const clock = { ...fakeClock(), __recreateFreezeSequences: true };
  const element = new FakeElement();
  element.textContent = 'Browse recent items';
  const stop = startSequence(element, sequence, clock);
  clock.tick(10000);
  assert.equal(element.textContent, 'Browse recent items');
  assert.equal(clock.pending(), 0);
  stop();
});
