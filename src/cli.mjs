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
  try {
    await import('./extract.mjs');
  } catch (error) {
    if (error?.code !== 'RECREATE_ACCESS_REQUIRED') throw error;
    console.error(error.message);
    process.exitCode = 2;
  }
}
