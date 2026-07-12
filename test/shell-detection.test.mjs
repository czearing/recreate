import assert from 'node:assert/strict';
import vm from 'node:vm';
import test from 'node:test';
import {
  authenticationShellRuntimeSource,
  isAuthenticationShell,
} from '../src/shell-detection.mjs';

test('detects bounded provider sign-in shells', () => {
  assert.equal(isAuthenticationShell({
    heading: 'Sign in to Copilot',
    bodyText: 'Sign in to Copilot. Sign in with Microsoft. Sign in with Apple.',
    authActionTexts: ['Sign in with Microsoft', 'Sign in with Apple'],
  }), true);
});

test('detects tiny single-action authentication fallbacks', () => {
  assert.equal(isAuthenticationShell({
    heading: 'Notebooks',
    bodyText: 'Notebooks\nSign in',
    authActionTexts: ['Sign in'],
  }), true);
});

test('does not reject applications with incidental sign-in controls', () => {
  assert.equal(isAuthenticationShell({
    heading: 'Microsoft Learn',
    bodyText: 'Documentation and training.'.repeat(200),
    authActionTexts: ['Sign in'],
  }), false);
});

test('does not reject a requested authentication control in rich content', () => {
  assert.equal(isAuthenticationShell({
    heading: 'Account settings',
    bodyText: 'Manage your account and connected applications.',
    authActionTexts: ['Sign in'],
  }), false);
});

test('runtime detector recognizes a document-level sign-in heading', () => {
  const element = (text, attributes = {}) => ({
    innerText: text,
    value: '',
    getAttribute: (name) => attributes[name] || '',
  });
  const document = {
    body: { innerText: 'Sign in to continue. Sign in.' },
    querySelectorAll: (selector) =>
      selector === 'h1,h2,[role="heading"]'
        ? [element('Sign in')]
        : [element('Sign in')],
  };
  assert.equal(vm.runInNewContext(
    authenticationShellRuntimeSource,
    { document },
  ), true);
});
