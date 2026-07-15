#!/usr/bin/env node

import { installSkill } from './skill-installer.mjs';
import { currentSkillInstructions } from './skill-content.mjs';

const [command, ...args] = process.argv.slice(2);

if (command === 'install') {
  await installSkill(args);
} else if (command === 'skill') {
  console.log(currentSkillInstructions());
} else if (command === 'help' || command === '--help' || command === '-h') {
  console.log(`Recreate

Usage:
  recreate <url> [options]
  recreate install [--copilot | --claude | --all]
  recreate skill
  recreate --version`);
} else {
  await import('./extract.mjs');
}
