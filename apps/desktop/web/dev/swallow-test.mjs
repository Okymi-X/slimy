// The swallowed-cursor arrow: drawn in the gel core while the catch holds, gone about 1.5 s after release.
// Drives dev/preview.html (real renderer, real timing) and sets window.__swallow like the page's catch phase.
// Screenshots land in $SHOTS (default /tmp/claude-1000/shots) as swallow-r<radius>-<step>-<n>.png.
// Usage: HARNESS_PORT=9613 node swallow-test.mjs [chromePort] [radius ...]   (needs python3 -m http.server in web/)
import { spawn } from 'node:child_process';
import { writeFileSync, mkdirSync, rmSync } from 'node:fs';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const [basePort = '9614', ...radii] = process.argv.slice(2);
const out = process.env.SHOTS ?? '/tmp/claude-1000/shots'; mkdirSync(out, { recursive: true });
const sizes = radii.length ? radii.map(Number) : [40, 64, 80];

async function run(radius, attempt = 0) {
  const chromePort = String(Number(basePort) + attempt * 10 + sizes.indexOf(radius) * 100);
  rmSync(`/tmp/claude-1000/chrome-${chromePort}`, { recursive: true, force: true });
  mkdirSync(`/tmp/claude-1000/chrome-${chromePort}`, { recursive: true });
  const url = `http://127.0.0.1:${process.env.HARNESS_PORT ?? 8765}/dev/preview.html?settle&radius=${radius}`;
  const chrome = spawn('google-chrome', ['--headless=new', '--no-sandbox', `--remote-debugging-port=${chromePort}`, '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
    '--window-size=500,500', `--user-data-dir=/tmp/claude-1000/chrome-${chromePort}`, url], { stdio: 'ignore' });
  let socket, id = 1; const pending = new Map();
  for (let i = 0; i < 60 && !socket; i++) {
    try {
      const t = (await (await fetch(`http://127.0.0.1:${chromePort}/json`)).json()).find((x) => x.type === 'page' && x.url.includes('preview'));
      if (t) { socket = new WebSocket(t.webSocketDebuggerUrl); await new Promise((r, j) => { socket.onopen = r; socket.onerror = j; }); }
    } catch { /* not up */ }
    if (!socket) await sleep(500);
  }
  if (!socket) { chrome.kill(); if (attempt < 3) return run(radius, attempt + 1); throw new Error('no devtools socket'); }
  socket.onmessage = (e) => { const m = JSON.parse(e.data); if (m.id && pending.has(m.id)) { pending.get(m.id)(m.result); pending.delete(m.id); } };
  const call = (method, params = {}) => new Promise((r) => { const i = id++; pending.set(i, r); socket.send(JSON.stringify({ id: i, method, params })); });
  const ev = async (x) => (await call('Runtime.evaluate', { expression: x, returnByValue: true, awaitPromise: true })).result?.value;
  const shot = async (name) => {
    const { data } = await call('Page.captureScreenshot', { format: 'png' });
    writeFileSync(`${out}/swallow-r${radius}-${name}.png`, Buffer.from(data, 'base64'));
    await ev(`(window.__f ??= {})['${name}'] = '${data}'`);
    return name;
  };
  // The per-pixel median of 5 frames (drifting bubbles drop out, the arrow stays), so the arrow can be told from the moving gel.
  const burst = async (name) => { const d = []; for (let i = 0; i < 5; i++) { d.push(await shot(`${name}-${i}`)); await sleep(180); } return d; };
  const changed = (A, B) => ev(`(async () => { const load = async (d) => { const img = new Image(); img.src = 'data:image/png;base64,' + d; await img.decode();
      const c = document.createElement('canvas'); c.width = img.width; c.height = img.height; const g = c.getContext('2d'); g.drawImage(img, 0, 0); return [g.getImageData(0, 0, c.width, c.height).data, c.width]; };
      const median = async (list) => { const ims = []; let W = 0; for (const name of list) { const [p, w] = await load(window.__f[name]); ims.push(p); W = w; }
        const m = new Uint8Array(ims[0].length); const v = new Array(ims.length);
        for (let i = 0; i < m.length; i++) { for (let k = 0; k < ims.length; k++) v[k] = ims[k][i]; v.sort((a, b) => a - b); m[i] = v[v.length >> 1]; } return [m, W]; };
      const [p, W] = await median(${JSON.stringify(A)}), [q] = await median(${JSON.stringify(B)});
      const bg = (a, i) => Math.abs(a[i] - 27) + Math.abs(a[i + 1] - 34) + Math.abs(a[i + 2] - 54) < 60 || Math.abs(a[i] - 32) + Math.abs(a[i + 1] - 41) + Math.abs(a[i + 2] - 61) < 60;
      let x0 = 1e9, y0 = 1e9, x1 = -1, y1 = -1; const H = p.length / 4 / W;
      for (let y = 150; y < H; y++) for (let x = 0; x < W; x++) if (!bg(q, (y * W + x) * 4)) { x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y); }
      // Lower middle of the body, under the mouth (where the arrow now sits).
      const cx = (x0 + x1) / 2, hw = (x1 - x0) * 0.15, top = y0 + (y1 - y0) * 0.7, bottom = y1 - (y1 - y0) * 0.06; let n = 0, white = 0, dark = 0;
      for (let y = Math.floor(top); y < bottom; y++) for (let x = Math.floor(cx - hw); x < cx + hw; x++) { const i = (y * W + x) * 4;
        if (Math.abs(p[i] - q[i]) + Math.abs(p[i + 1] - q[i + 1]) + Math.abs(p[i + 2] - q[i + 2]) > 45) { n++; if (p[i] > 150 && p[i + 2] > 190) white++; if (p[i] < q[i] - 25) dark++; } }
      return { n, white, dark, box: [x0, y0, x1, y1] }; })()`);
  await sleep(4000);
  await burst('0-before'); // only for the eye: the pose before the catch
  await ev('window.__swallow = "hold"'); await sleep(1200);
  const held = await burst('1-hold');
  await ev('window.__swallow = "none"'); await sleep(1500);
  const after = await burst('2-after');
  await sleep(1500);
  const later = await burst('3-later'); // an arrow still fading at 1.5 s would be gone by now: the two must match
  const dHold = await changed(held, after), dAfter = await changed(later, after);
  chrome.kill(); await sleep(500);
  // Hold: white-filled, dark-outlined pixels appear that the released frames lack (the gel itself also drifts, so only pale pixels
  // count); 1.5 s after release no pale pixel differs from the frames 1.5 s later, so nothing is left fading.
  const ok = dHold.white >= radius * 0.1 && dAfter.white <= 2;
  console.log(`radius ${radius}: hold vs released: ${dHold.n} changed pixels (arrow fill-like ${dHold.white}, dark ${dHold.dark}); released 1.5 s vs 3 s: ${dAfter.n} (white ${dAfter.white}, dark ${dAfter.dark}); body box ${dHold.box} -> ${ok ? 'PASS' : 'FAIL'}`);
  return ok;
}
let all = true;
for (const r of sizes) { await sleep(300); all = (await run(r)) && all; }
console.log(`RESULT ${all ? 'PASS' : 'FAIL'}`);
process.exit(all ? 0 : 1);
