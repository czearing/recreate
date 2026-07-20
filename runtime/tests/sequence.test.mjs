import assert from 'node:assert/strict';
import test from 'node:test';
import { applySequenceValue, startSequence } from '../sequence.mjs';
import { FakeElement, fakeClock } from './support.mjs';

const sequence = {
  attribute: 'textContent',
  steps: [
    { value: 'Launch plan', delay_ms: 4000 },
    { value: 'Competitive analysis', delay_ms: 3500 },
    { value: 'Across your notebooks', delay_ms: 4250 },
    { value: 'Proactive drafts', delay_ms: 3000 },
  ],
};

test('plays irregular steps and loops twice without wall-clock waits', () => {
  const clock = fakeClock();
  const element = new FakeElement();
  const stop = startSequence(element, sequence, clock);
  assert.equal(element.textContent, 'Launch plan');
  for (const [delay, value] of [
    [4000, 'Competitive analysis'],
    [3500, 'Across your notebooks'],
    [4250, 'Proactive drafts'],
    [3000, 'Launch plan'],
    [4000, 'Competitive analysis'],
    [3500, 'Across your notebooks'],
    [4250, 'Proactive drafts'],
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
  element.textContent = 'Across your notebooks';
  const stop = startSequence(element, sequence, clock);
  assert.equal(element.textContent, 'Across your notebooks');
  clock.tick(4250);
  assert.equal(element.textContent, 'Proactive drafts');
  stop();
});

test('does not schedule incomplete sequences', () => {
  const clock = fakeClock();
  const stop = startSequence(new FakeElement(), { attribute: 'title', steps: [] }, clock);
  stop();
  assert.equal(clock.pending(), 0);
});
