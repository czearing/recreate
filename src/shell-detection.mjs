export function isAuthenticationShell({
  heading = '',
  bodyText = '',
  authActionTexts = [],
}) {
  const compactBody = bodyText.replace(/\s+/g, ' ').trim();
  const compactHeading = heading.replace(/\s+/g, ' ').trim();
  const authActions = authActionTexts.filter((text) =>
    /^sign in(?:\s+(?:to|with)\b.*)?$/i.test(text.replace(/\s+/g, ' ').trim())
  );
  let residualBody = compactBody.toLowerCase();
  for (const text of [compactHeading, ...authActions]) {
    const token = text.toLowerCase();
    if (token.length >= 3) residualBody = residualBody.split(token).join(' ');
  }
  residualBody = residualBody.replace(/[^a-z0-9]+/g, ' ').trim();
  return (
    compactBody.length > 0 &&
    compactBody.length < 2000 &&
    (
      /^sign in(?:\s+to\b.*)?$/i.test(compactHeading) ||
      authActions.length >= 2 ||
      (
        compactBody.length < 300 &&
        authActions.length >= 1 &&
        residualBody.length < 40
      )
    )
  );
}

export const authenticationShellRuntimeSource = `(() => {
  const heading = Array.from(document.querySelectorAll(
    'h1,h2,[role="heading"]'
  )).map(element => (element.innerText || '').trim()).join(' ');
  const bodyText = document.body?.innerText || '';
  const authActionTexts = Array.from(document.querySelectorAll(
    'button,a,[role="button"],input[type="submit"]'
  )).map(element => (
    element.innerText ||
    element.value ||
    element.getAttribute('aria-label') ||
    ''
  ).trim());
  return (${isAuthenticationShell.toString()})({
    heading,
    bodyText,
    authActionTexts
  });
})()`;
