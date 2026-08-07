import {createServer} from 'node:http';
import {readFile, stat} from 'node:fs/promises';
import {join, extname, normalize} from 'node:path';

const ROOT = process.argv[2];
const SITES = process.argv[3].split(',');
const BASE = Number(process.argv[4]);
const TYPES = {'.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.gif': 'image/gif', '.webp': 'image/webp', '.woff': 'font/woff', '.woff2': 'font/woff2', '.ttf': 'font/ttf', '.ico': 'image/x-icon'};

// One server per site, each rooted at that site's dist, so the absolute
// /assets/... references the build emits resolve.
SITES.forEach((site, i) => {
  const root = join(ROOT, site, 'react', 'dist');
  createServer(async (req, res) => {
    try {
      const url = decodeURIComponent(req.url.split('?')[0]);
      let file = normalize(join(root, url));
      if (!file.startsWith(normalize(root))) throw new Error('escape');
      try { if ((await stat(file)).isDirectory()) file = join(file, 'index.html'); } catch { file = join(root, 'index.html'); }
      const body = await readFile(file);
      res.writeHead(200, {'content-type': TYPES[extname(file)] || 'application/octet-stream', 'cache-control': 'no-store'});
      res.end(body);
    } catch { res.writeHead(404); res.end('not found'); }
  }).listen(BASE + i, () => console.log(`${site} -> http://127.0.0.1:${BASE + i}/`));
});
