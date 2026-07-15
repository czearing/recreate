#!/usr/bin/env node
import path from 'node:path';
import { buildReactProject } from './src/react-source/project.mjs';

const args = Object.fromEntries(process.argv.slice(2).flatMap((arg, index, values) =>
  arg.startsWith('--') ? [[arg.slice(2), values[index + 1]]] : []));
if (!args.spec || !args.out) {
  throw new Error('Usage: node build-react.mjs --spec <recreate-output> --out <react-project>');
}
const result = buildReactProject({
  specDir: path.resolve(args.spec),
  outDir: path.resolve(args.out),
  maxNodes: Number.parseInt(args['max-nodes'] || '20', 10),
});
console.log(JSON.stringify(result, null, 2));
