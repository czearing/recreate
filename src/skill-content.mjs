export const latestRecreateCommand =
  'npx --yes --prefer-online recreate-cli@latest';

export function installedSkillContent() {
  return `---
name: recreate
description: Capture and recreate a live interface with its structure, styling, responsive behavior, assets, and interactions. Use when asked to copy, rebuild, inspect, or validate a website or application interface.
license: MIT
---

Run this command first every time the skill is used:

\`${latestRecreateCommand} skill\`

Follow the current workflow printed by the command. Do not replace structured
capture with screenshot guessing. The command checks npm for the newest Recreate
release before loading the workflow.
`;
}

export function currentSkillInstructions() {
  return `Current Recreate workflow:

1. Get the source URL and the destination repository or output path.
2. Start or reuse a Chromium browser with remote debugging on port 9222, open
   the source URL, and briefly inspect the rendered page before capture.
3. Decide from the page as a whole whether it is the requested interface or an
   access step in front of it. Do not decide from fixed words, selectors, URL
   patterns, or the mere presence of account controls.
   - If it is clearly the requested interface, continue.
   - If it clearly blocks the requested interface, ask the user to complete
     access in the open browser tab. Wait, then inspect the same tab again.
   - If the access page itself may be the requested interface, ask whether to
     recreate that page or wait for the page behind it.
4. Keep credentials and session data in the browser. Never ask the user to copy
   them into chat or the terminal.
5. Capture the inspected tab by exact target ID:
   ${latestRecreateCommand} --reuse --target <target-id> --out recreate-output
6. Read recreate-output/implementation.json first. Open detailed evidence only
   for the component or state currently being implemented.
7. Rebuild the interface natively. Preserve captured content, layout, assets,
   responsive behavior, motion, and interactions.
8. Validate the result against recreate-output/acceptance-matrix.json before
   declaring the work complete.
`;
}
