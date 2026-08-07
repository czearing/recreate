import http from 'node:http';

// Serves synthetic pages whose DOM node count is set by the URL path: /n/<count>.
// Each node carries a distinct class so the capture page script must serialise a
// comparable amount of computed style per node, which is what fills the CDP
// response message that the 64 MiB cap applies to.
function page(count) {
  const parts = [];
  for (let i = 0; i < count; i += 1) {
    parts.push(
      `<div class="c${i} row-${i % 7}" id="n${i}" data-idx="${i}">` +
        `<span class="lbl">Item ${i}</span>` +
        `</div>`,
    );
  }
  return `<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>ladder ${count}</title>
<style>
body{margin:0;font:14px/1.4 system-ui,sans-serif}
div{display:flex;align-items:center;padding:2px 6px;border-bottom:1px solid #eee}
.row-0{background:#fafafa}.row-1{background:#f5f5f5}.row-2{background:#f0f0f0}
.row-3{background:#ebebeb}.row-4{background:#e6e6e6}.row-5{background:#e1e1e1}
.row-6{background:#dcdcdc}
.lbl{flex:1 1 auto;color:#333}
@media (max-width:768px){div{padding:1px 3px}}
@media (max-width:480px){.lbl{font-size:12px}}
</style></head><body><main>${parts.join('')}</main></body></html>`;
}

const port = Number(process.argv[2] || 8811);
http
  .createServer((req, res) => {
    const match = /^\/n\/(\d+)/.exec(req.url || '');
    if (!match) {
      res.writeHead(404, { 'content-type': 'text/plain' });
      res.end('use /n/<count>');
      return;
    }
    const body = page(Number(match[1]));
    res.writeHead(200, {
      'content-type': 'text/html; charset=utf-8',
      'content-length': Buffer.byteLength(body),
    });
    res.end(body);
  })
  .listen(port, '127.0.0.1', () => {
    console.log(`ladder server on ${port}`);
  });
