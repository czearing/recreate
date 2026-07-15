export const latestRecreateCommand =
  'node <recreate-skill-directory>/run.mjs';

export function installedSkillContent() {
  return `---
name: recreate
description: Capture and recreate a live interface with its structure, styling, responsive behavior, assets, and interactions. Use when asked to copy, rebuild, inspect, or validate a website or application interface.
license: MIT
---

From this skill directory, run this command first every time the skill is used:

\`node run.mjs skill\`

Follow the current workflow printed by the command. Do not replace structured
capture with screenshot guessing. The runner checks GitHub Releases for the
newest stable Recreate package before loading the workflow.
`;
}

export function currentSkillInstructions() {
  return `Current Recreate workflow:

1. Get the source URL and the destination repository or output path.
2. Handle the browser setup yourself. Do not ask the user to start, configure,
   or locate a browser.
   - Probe http://127.0.0.1:9222/json/version.
   - If unavailable, locate installed Chrome, Edge, or Chromium and launch it
     with --remote-debugging-port=9222 and a persistent Recreate profile.
   - Open the source URL in that browser through CDP and keep the target ID.
   Do not substitute HTTP fetches, raw HTML, or response text for browser
   inspection.
3. Briefly inspect the rendered page in that browser before capture.
4. Decide from the page as a whole whether it is the requested interface or an
   access step in front of it. Do not decide from fixed words, selectors, URL
   patterns, or the mere presence of account controls.
   - If it is clearly the requested interface, continue.
   - If it clearly blocks the requested interface, ask the user to complete
     access in the open browser tab. Wait, then inspect the same tab again.
   - If the access page itself may be the requested interface, ask whether to
     recreate the visible page or wait for the page behind it. Ask this as one
     short natural question without internal option names.
   This page-intent question is the only browser setup question permitted.
5. Keep credentials and session data in the browser. Never ask the user to copy
   them into chat or the terminal.
6. Capture the inspected tab by exact target ID and the same CDP endpoint:
   ${latestRecreateCommand} --reuse --target <target-id> --cdp-url http://127.0.0.1:9222 --out recreate-output
7. Read recreate-output/implementation.json first. Open detailed evidence only
   for the component or state currently being implemented.
8. Rebuild the interface natively. Preserve captured content, layout, assets,
   responsive behavior, motion, and interactions.
9. Validate the result against recreate-output/acceptance-matrix.json before
   declaring the work complete.
`;
}
