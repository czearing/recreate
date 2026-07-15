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
2. Start or reuse a Chromium browser with remote debugging on port 9222. Handle
   this in the terminal instead of asking the user to configure it manually.
3. Run:
   ${latestRecreateCommand} <url> --out recreate-output
4. If Recreate returns RECREATE_ACCESS_REQUIRED:
   - Tell the user: "Access is required in the browser Recreate opened. Complete
     it there. Recreate will continue automatically."
   - Keep credentials and session data inside the browser.
   - Keep the running command alive and wait for it to continue. Do not open a
     new tab or start a second capture.
   - If the command times out, read recreate-output/access-required.json and run
     its resume.display command against the same browser tab.
5. Read recreate-output/implementation.json first. Open detailed evidence only
   for the component or state currently being implemented.
6. Rebuild the interface natively. Preserve captured content, layout, assets,
   responsive behavior, motion, and interactions.
7. Validate the result against recreate-output/acceptance-matrix.json before
   declaring the work complete.

Use --reuse with an exact tab match when the source is already open or requires
an authenticated browser session.`;
}
