// Measures what a width-fidelity check would COST per page, against the
// 4400ms comparison deadline. Times the two components separately:
// the viewport resize (forced layout) and the rect collection pass.
import { readFileSync } from 'node:fs';

const CDP = process.argv[2];
const URL = process.argv[3];
const WIDTHS = [400, 450, 480, 600, 661, 800, 1076, 1440];

const rpc = async (ws, id, method, params, sessionId) =>
  new Promise((res, rej) => {
    const h = (e) => {
      const m = JSON.parse(e.data);
      if (m.id === id) { ws.removeEventListener('message', h); m.error ? rej(new Error(JSON.stringify(m.error))) : res(m.result); }
    };
    ws.addEventListener('message', h);
    ws.send(JSON.stringify({ id, method, params, sessionId }));
  });

const COLLECT = `(() => {
  const out = [];
  const els = document.querySelectorAll('*');
  for (const el of els) {
    const r = el.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) continue;
    const t = (el.textContent || '').trim().slice(0, 80);
    if (!t) continue;
    out.push([t, Math.round(r.left), Math.round(r.width)]);
  }
  return out.length;
})()`;

const main = async () => {
  const v = await (await fetch(`${CDP}/json/version`)).json();
  const ws = new WebSocket(v.webSocketDebuggerUrl);
  await new Promise((r) => ws.addEventListener('open', r));
  let id = 1;
  const t = await rpc(ws, id++, 'Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await rpc(ws, id++, 'Target.attachToTarget', { targetId: t.targetId, flatten: true });

  await rpc(ws, id++, 'Page.enable', {}, sessionId);
  await rpc(ws, id++, 'Page.navigate', { url: URL }, sessionId);
  await new Promise((r) => setTimeout(r, 6000));

  const evaluate = (expr) => rpc(ws, id++, 'Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: false }, sessionId);

  // warm up so first-load cost is not attributed to the check
  await rpc(ws, id++, 'Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false }, sessionId);
  await evaluate(COLLECT);

  let resizeMs = 0, collectMs = 0, elems = 0;
  for (const w of WIDTHS) {
    let a = Date.now();
    await rpc(ws, id++, 'Emulation.setDeviceMetricsOverride', { width: w, height: 900, deviceScaleFactor: 1, mobile: false }, sessionId);
    resizeMs += Date.now() - a;
    a = Date.now();
    const r = await evaluate(COLLECT);
    collectMs += Date.now() - a;
    elems = r.result.value;
  }
  console.log(JSON.stringify({
    url: URL, widths: WIDTHS.length, elementsMeasuredAtLastWidth: elems,
    resizeTotalMs: resizeMs, collectTotalMs: collectMs,
    totalOneSideMs: resizeMs + collectMs,
    perWidthMs: +((resizeMs + collectMs) / WIDTHS.length).toFixed(1),
  }));
  await rpc(ws, id++, 'Target.closeTarget', { targetId: t.targetId });
  ws.close();
};
main().catch((e) => { console.error('ERR', e.message); process.exit(1); });
