import assert from 'node:assert/strict';
import test from 'node:test';
import {
  replayControlValue,
  setControlValue,
  submitIntent,
} from '../textarea.mjs';
import { FakeElement } from './support.mjs';

class FakeTextArea extends FakeElement {}
Object.defineProperty(FakeTextArea.prototype, 'value', {
  set(value) { this._value = `textarea:${value}`; },
  get() { return this._value; },
});

class FakeInput extends FakeElement {}
Object.defineProperty(FakeInput.prototype, 'value', {
  set(value) { this._value = `input:${value}`; },
  get() { return this._value; },
});

class FakeEvent {
  constructor(type) { this.type = type; }
}

const environment = {
  HTMLTextAreaElement: FakeTextArea,
  HTMLInputElement: FakeInput,
  Event: FakeEvent,
};

test('uses the textarea setter instead of the input setter', () => {
  const textarea = new FakeTextArea();
  const input = new FakeInput();
  setControlValue(textarea, 'draft', environment);
  setControlValue(input, 'query', environment);
  assert.equal(textarea.value, 'textarea:draft');
  assert.equal(input.value, 'input:query');
});

test('replays input and change in browser order', () => {
  const textarea = new FakeTextArea();
  replayControlValue(textarea, 'draft', environment);
  assert.deepEqual(textarea.events, ['input', 'change']);
});

test('separates submit, newline, composition, and unrelated keys', () => {
  assert.equal(submitIntent({ key: 'Enter', shiftKey: false, isComposing: false }), 'submit');
  assert.equal(submitIntent({ key: 'Enter', shiftKey: true, isComposing: false }), 'newline');
  assert.equal(submitIntent({ key: 'Enter', shiftKey: false, isComposing: true }), 'none');
  assert.equal(submitIntent({ key: 'Tab', shiftKey: false, isComposing: false }), 'none');
});

test('rejects controls without a native value setter', () => {
  assert.throws(() => setControlValue(new FakeElement(), 'value', environment), TypeError);
});
