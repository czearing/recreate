export function buildResumeArguments(argv, { targetId, outDir }) {
  const valueFlags = new Set(['--match', '--out', '--target', '--url']);
  const preserved = [];
  for (let index = 0; index < argv.length; index++) {
    const argument = argv[index];
    if (index === 0 && !argument.startsWith('--')) continue;
    if (argument === '--reuse') continue;
    if (valueFlags.has(argument)) {
      if (argv[index + 1] && !argv[index + 1].startsWith('--')) index++;
      continue;
    }
    preserved.push(argument);
  }
  return ['--reuse', '--target', targetId, '--out', outDir, ...preserved];
}

export function buildAccessMarker({
  argv,
  currentUrl,
  outDir,
  requestedUrl,
  requirement,
  targetId,
}) {
  const captureArgs = buildResumeArguments(argv, { targetId, outDir });
  const args = [
    '--yes',
    '--prefer-online',
    'recreate-cli@latest',
    ...captureArgs,
  ];
  return {
    code: 'RECREATE_ACCESS_REQUIRED',
    currentUrl,
    kind: requirement.kind,
    reason: requirement.reason,
    requestedUrl,
    resume: {
      args,
      command: 'npx',
      display: ['npx', ...args.map((argument) => JSON.stringify(argument))]
        .join(' '),
    },
    targetId,
  };
}
