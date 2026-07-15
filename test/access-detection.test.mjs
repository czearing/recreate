import assert from 'node:assert/strict';
import test from 'node:test';
import {
  classifyAccessRequirement,
  hasAuthenticationProtocol,
} from '../src/access-detection.mjs';
import {
  buildAccessMarker,
  buildResumeArguments,
} from '../src/access-resume.mjs';

const requestedUrl = 'https://app.example.com/private';

test('detects document authentication and authorization responses', () => {
  assert.deepEqual(
    classifyAccessRequirement({
      requestedUrl,
      currentUrl: requestedUrl,
      networkRequests: [
        { type: 'Document', isMainFrame: true, status: 302 },
        { type: 'Document', isMainFrame: true, status: 401 },
      ],
    }),
    { kind: 'authentication', reason: 'document-challenge' },
  );
  assert.deepEqual(
    classifyAccessRequirement({
      requestedUrl,
      currentUrl: requestedUrl,
      networkRequests: [
        { type: 'Document', isMainFrame: true, status: 403 },
      ],
    }),
    { kind: 'authorization', reason: 'document-denied' },
  );
});

test('detects protocol redirects without provider or text matching', () => {
  const redirect =
    'https://identity.example.net/authorize?client_id=x&redirect_uri=y&response_type=code';
  assert.equal(hasAuthenticationProtocol(redirect), true);
  assert.deepEqual(
    classifyAccessRequirement({
      requestedUrl,
      currentUrl: redirect,
      networkRequests: [
        {
          type: 'Document',
          isMainFrame: true,
          url: requestedUrl,
          redirectToUrl: redirect,
        },
      ],
    }),
    { kind: 'authentication', reason: 'federated-redirect' },
  );
});

test('detects visible credential controls', () => {
  assert.deepEqual(
    classifyAccessRequirement({
      requestedUrl,
      currentUrl: 'https://identity.example.net/',
      domState: { hasCredentialControls: true },
    }),
    { kind: 'authentication', reason: 'credential-controls' },
  );
});

test('uses application API challenges only after the page settles', () => {
  const networkRequests = [
    {
      type: 'Fetch',
      url: 'https://app.example.com/api/session',
      status: 401,
    },
  ];
  assert.equal(
    classifyAccessRequirement({
      requestedUrl,
      currentUrl: requestedUrl,
      networkRequests,
      domState: { hasApplicationSurface: false },
    }),
    undefined,
  );
  assert.deepEqual(
    classifyAccessRequirement({
      requestedUrl,
      currentUrl: requestedUrl,
      networkRequests,
      domState: { hasApplicationSurface: false },
      includeApiChallenges: true,
    }),
    { kind: 'authentication', reason: 'application-challenge' },
  );
});

test('ignores login-like text without structural evidence', () => {
  assert.equal(
    classifyAccessRequirement({
      requestedUrl,
      currentUrl: requestedUrl,
      domState: {
        bodyText: 'Login or sign in to continue',
        hasApplicationSurface: true,
      },
    }),
    undefined,
  );
});

test('preserves capture options in the resume arguments', () => {
  assert.deepEqual(
    buildResumeArguments(
      [
        requestedUrl,
        '--out',
        'old-output',
        '--crawl',
        '--viewports',
        '1440x900,390x844',
      ],
      { targetId: 'target-1', outDir: 'C:\\capture' },
    ),
    [
      '--reuse',
      '--target',
      'target-1',
      '--out',
      'C:\\capture',
      '--crawl',
      '--viewports',
      '1440x900,390x844',
    ],
  );
});

test('builds a directly executable and credential-free resume marker', () => {
  const marker = buildAccessMarker({
    argv: [requestedUrl, '--crawl'],
    currentUrl: 'https://identity.example.net/?code=redacted',
    outDir: 'C:\\capture path',
    requestedUrl,
    requirement: { kind: 'authentication', reason: 'federated-redirect' },
    targetId: 'target-1',
  });
  assert.equal(marker.resume.command, 'npx');
  assert.equal(marker.resume.args[2], 'recreate-cli@latest');
  assert.match(marker.resume.display, /^npx /);
  assert.doesNotMatch(JSON.stringify(marker), /cookie|password|token/i);
});
