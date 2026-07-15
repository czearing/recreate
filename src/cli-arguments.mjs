export function getPositionalUrl(argv) {
  const firstArgument = argv[0];
  return firstArgument && !firstArgument.startsWith('--') ? firstArgument : '';
}
