import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';

export function createFigmaValueStore(evidenceDir) {
  const values = {};
  const valueKeys = new Map();
  const reference = (value) => {
    if (value == null) return undefined;
    const json = JSON.stringify(value);
    if (json.length < 80) return value;
    let key = valueKeys.get(json);
    if (!key) {
      key = createHash('sha256').update(json).digest('hex').slice(0, 20);
      valueKeys.set(json, key);
      values[key] = value;
    }
    return { $ref: key };
  };
  const write = () => {
    const directory = path.join(evidenceDir, 'values');
    fs.mkdirSync(directory, { recursive: true });
    const shards = {};
    for (const [key, value] of Object.entries(values)) {
      const prefix = key[0];
      shards[prefix] ||= {};
      shards[prefix][key] = value;
    }
    const files = [];
    for (const [prefix, shardValues] of Object.entries(shards)) {
      const filename = `${prefix}.json`;
      fs.writeFileSync(
        path.join(directory, filename),
        JSON.stringify({ values: shardValues }),
      );
      files.push(`evidence/figma/values/${filename}`);
    }
    return {
      shardPrefixLength: 1,
      pattern: 'evidence/figma/values/<first-hash-character>.json',
      shards: files.sort(),
    };
  };
  return { reference, write };
}
