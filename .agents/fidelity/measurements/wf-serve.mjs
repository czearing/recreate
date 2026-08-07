// Serves each built recreation at a filesystem root on its own port.
// Root serving matters: a build emitting absolute asset URLs renders nothing
// under a subpath and would return a silent clean zero.
import { createServer } from 'node:http';
import { existsSync, createReadStream, statSync } from 'node:fs';
import { join, extname, normalize } from 'node:path';

const ROOT = process.argv[2];
const SITES = [
  ['reactdev', 8841], ['sveltedev', 8842], ['vuejs', 8843],
  ['cern', 8844], ['danluu', 8845], ['nprtext', 8846],
  ['lobsters', 8847], ['gnu', 8848], ['w3c', 8849], ['sourcehut', 8850],
];
const TYPES = {
  '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript',
  '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png',
  '.jpg': 'image/jpeg', '.gif': 'image/gif', '.json': 'application/json',
  '.woff': 'font/woff', '.woff2': 'font/woff2', '.ico': 'image/x-icon',
};

for (const [key, port] of SITES) {
  const dist = join(ROOT, key, 'react', 'dist');
  if (!existsSync(dist)) { console.log(`${key} NO DIST`); continue; }
  createServer((req, res) => {
    let url = decodeURIComponent(req.url.split('?')[0]);
    if (url === '/') url = '/index.html';
    let fp = normalize(join(dist, url));
    if (!fp.startsWith(dist) || !existsSync(fp) || statSync(fp).isDirectory()) fp = join(dist, 'index.html');
    res.writeHead(200, { 'Content-Type': TYPES[extname(fp)] || 'application/octet-stream' });
    createReadStream(fp).pipe(res);
  }).listen(port, '127.0.0.1', () => console.log(`${key} -> ${port}`));
}
