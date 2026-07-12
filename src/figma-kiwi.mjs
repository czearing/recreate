import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { decodeBinarySchema, compileSchema } = require('kiwi-schema');
const pako = require('pako');
const fzstd = require('fzstd');

const isZstd = (chunk) =>
  chunk.length >= 4 &&
  chunk[0] === 0x28 &&
  chunk[1] === 0xb5 &&
  chunk[2] === 0x2f &&
  chunk[3] === 0xfd;

const decompress = (chunk) =>
  isZstd(chunk) ? fzstd.decompress(chunk) : pako.inflateRaw(chunk);

export function decodeFigmaKiwi(bytes) {
  if (new TextDecoder().decode(bytes.slice(0, 8)) !== 'fig-kiwi') {
    throw new Error('Figma canvas response did not contain a fig-kiwi file.');
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const version = view.getUint32(8, true);
  let offset = 12;
  const chunks = [];
  while (offset + 4 <= bytes.length) {
    const size = view.getUint32(offset, true);
    offset += 4;
    if (offset + size > bytes.length) {
      throw new Error(`Invalid fig-kiwi chunk at ${offset - 4}.`);
    }
    chunks.push(bytes.slice(offset, offset + size));
    offset += size;
  }
  if (chunks.length < 2) throw new Error('Figma canvas response omitted Kiwi chunks.');
  const schema = decodeBinarySchema(decompress(chunks[0]));
  const message = compileSchema(schema).decodeMessage(decompress(chunks[1]));
  if (!Array.isArray(message.nodeChanges) || !message.nodeChanges.length) {
    throw new Error('Decoded Figma canvas contained no design nodes.');
  }
  return {
    version,
    schemaDefinitionCount: schema.definitions?.length || 0,
    chunkSizes: chunks.map((chunk) => chunk.length),
    message,
  };
}
