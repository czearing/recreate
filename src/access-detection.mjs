const oidcKeys = new Set([
  'client_id',
  'code_challenge',
  'nonce',
  'redirect_uri',
  'response_type',
]);

function parsedUrl(value) {
  try {
    return new URL(value);
  } catch {
    return undefined;
  }
}

function sameOrigin(left, right) {
  const leftUrl = parsedUrl(left);
  const rightUrl = parsedUrl(right);
  return Boolean(leftUrl && rightUrl && leftUrl.origin === rightUrl.origin);
}

export function hasAuthenticationProtocol(rawUrl) {
  const url = parsedUrl(rawUrl);
  if (!url) return false;
  const keys = new Set(
    [...url.searchParams.keys()].map((key) => key.toLowerCase()),
  );
  if (keys.has('samlrequest')) return true;
  if (keys.has('wtrealm') && keys.has('wa')) return true;
  return [...oidcKeys].filter((key) => keys.has(key)).length >= 2;
}

export function classifyAccessRequirement({
  requestedUrl,
  currentUrl,
  networkRequests = [],
  domState = {},
  includeApiChallenges = false,
}) {
  const mainDocuments = networkRequests.filter(
    (request) => request.type === 'Document' && request.isMainFrame,
  );
  if (
    mainDocuments.some(
      (request) => request.hasWwwAuthenticate || request.status === 401,
    )
  ) {
    return { kind: 'authentication', reason: 'document-challenge' };
  }
  if (mainDocuments.some((request) => request.status === 403)) {
    return { kind: 'authorization', reason: 'document-denied' };
  }

  const federatedRedirect = networkRequests.find(
    (request) =>
      request.isMainFrame &&
      request.redirectToUrl &&
      !sameOrigin(request.url, request.redirectToUrl) &&
      hasAuthenticationProtocol(request.redirectToUrl),
  );
  if (
    federatedRedirect ||
    (
      currentUrl &&
      !sameOrigin(requestedUrl, currentUrl) &&
      hasAuthenticationProtocol(currentUrl)
    )
  ) {
    return { kind: 'authentication', reason: 'federated-redirect' };
  }

  if (domState.hasCredentialControls) {
    return { kind: 'authentication', reason: 'credential-controls' };
  }
  if (
    domState.hasIdentityInput &&
    currentUrl &&
    !sameOrigin(requestedUrl, currentUrl)
  ) {
    return { kind: 'authentication', reason: 'external-identity-input' };
  }

  if (includeApiChallenges && !domState.hasApplicationSurface) {
    const relevantApi = networkRequests.find(
      (request) =>
        ['Fetch', 'XHR'].includes(request.type) &&
        (
          sameOrigin(request.url, requestedUrl) ||
          sameOrigin(request.url, currentUrl)
        ) &&
        [401, 403].includes(request.status),
    );
    if (relevantApi?.status === 401) {
      return { kind: 'authentication', reason: 'application-challenge' };
    }
    if (relevantApi?.status === 403) {
      return { kind: 'authorization', reason: 'application-denied' };
    }
  }

  return undefined;
}

export const accessDomRuntimeSource = `(() => {
  const visible = element => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return (
      rect.width > 0 &&
      rect.height > 0 &&
      style.display !== 'none' &&
      style.visibility !== 'hidden' &&
      Number(style.opacity || 1) > 0
    );
  };
  const controls = Array.from(document.querySelectorAll('input')).filter(visible);
  const credentialTokens = new Set([
    'current-password',
    'new-password',
    'one-time-code',
    'username',
    'webauthn'
  ]);
  const hasCredentialControls = controls.some(element => {
    const autocomplete = (element.autocomplete || '')
      .split(/\\s+/)
      .map(value => value.toLowerCase());
    return (
      element.type === 'password' ||
      autocomplete.some(value => credentialTokens.has(value))
    );
  });
  const hasIdentityInput = controls.some(element =>
    element.type === 'email' ||
    (element.autocomplete || '').split(/\\s+/).includes('email')
  );
  const hasApplicationSurface = Array.from(document.querySelectorAll(
    'main,[role="main"],[role="application"]'
  )).filter(visible).some(element => {
    const rect = element.getBoundingClientRect();
    const area = rect.width * rect.height;
    const controlCount = element.querySelectorAll(
      'a,button,input,select,textarea,[role="button"]'
    ).length;
    return (
      area > innerWidth * innerHeight * 0.2 &&
      ((element.innerText || '').trim().length > 300 || controlCount > 4)
    );
  });
  return {
    currentUrl: location.href,
    hasApplicationSurface,
    hasCredentialControls,
    hasIdentityInput
  };
})()`;
