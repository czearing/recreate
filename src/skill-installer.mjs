import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { installedSkillContent } from './skill-content.mjs';

const clients = {
  copilot: {
    command: 'copilot',
    directory: '.copilot',
    reload: 'Start a new session or run /skills reload.',
  },
  claude: {
    command: 'claude',
    directory: '.claude',
    reload: 'Start a new session. Existing sessions detect updates when skills were already loaded.',
  },
};

function executableExists(command, env = process.env) {
  const extensions =
    process.platform === 'win32'
      ? String(env.PATHEXT || '.EXE;.CMD;.BAT;.COM').split(';')
      : [''];
  return String(env.PATH || '')
    .split(path.delimiter)
    .filter(Boolean)
    .some((directory) =>
      extensions.some((extension) =>
        fs.existsSync(path.join(directory, `${command}${extension.toLowerCase()}`)) ||
        fs.existsSync(path.join(directory, `${command}${extension.toUpperCase()}`)),
      ),
    );
}

export function selectInstallTargets(
  args,
  homeDirectory = process.env.RECREATE_HOME || os.homedir(),
  env = process.env,
) {
  const knownFlags = new Set(['--all', '--copilot', '--claude']);
  const unknown = args.filter((argument) => !knownFlags.has(argument));
  if (unknown.length) {
    throw new Error(`Unknown install option: ${unknown.join(', ')}`);
  }
  if (args.includes('--all')) return Object.keys(clients);

  const requested = Object.keys(clients).filter((name) =>
    args.includes(`--${name}`),
  );
  if (requested.length) return requested;

  const detected = Object.entries(clients)
    .filter(
      ([, client]) =>
        fs.existsSync(path.join(homeDirectory, client.directory)) ||
        executableExists(client.command, env),
    )
    .map(([name]) => name);
  if (detected.length) return detected;

  throw new Error(
    'No Copilot or Claude installation was detected. Use --copilot, --claude, or --all.',
  );
}

export function writeSkill(
  clientName,
  homeDirectory = process.env.RECREATE_HOME || os.homedir(),
) {
  const client = clients[clientName];
  if (!client) throw new Error(`Unsupported client: ${clientName}`);

  const skillDirectory = path.join(
    homeDirectory,
    client.directory,
    'skills',
    'recreate',
  );
  const skillPath = path.join(skillDirectory, 'SKILL.md');
  if (
    fs.existsSync(skillDirectory) &&
    fs.lstatSync(skillDirectory).isSymbolicLink()
  ) {
    throw new Error(`Refusing to replace linked skill directory: ${skillDirectory}`);
  }
  fs.mkdirSync(skillDirectory, { recursive: true });
  fs.writeFileSync(skillPath, installedSkillContent(), 'utf8');
  return { clientName, skillPath, reload: client.reload };
}

export function installSkill(
  args,
  homeDirectory = process.env.RECREATE_HOME || os.homedir(),
) {
  const targets = selectInstallTargets(args, homeDirectory);
  const installed = targets.map((target) => writeSkill(target, homeDirectory));

  console.log('Recreate skill installed:');
  for (const result of installed) {
    console.log(`- ${result.clientName}: ${result.skillPath}`);
  }
  console.log('');
  console.log(
    'Every use checks npm for recreate-cli@latest before running Recreate.',
  );
  for (const result of installed) console.log(`- ${result.clientName}: ${result.reload}`);
  return installed;
}
