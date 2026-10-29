// Shared by the headless tests: launches Chrome on dev/app-harness.html and drives it over the DevTools protocol.
// Needs `python3 -m http.server 8765` running in apps/desktop/web.
import { spawn } from 'node:child_process';
import { writeFileSync, mkdirSync } from 'node:fs';

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export async function launch({ provider, port, tag, out = process.env.SHOTS ?? '/tmp/claude-1000/shots', soul = '' }) {
  mkdirSync(out, { recursive: true });
  const url = `http://localhost:${process.env.HARNESS_PORT ?? 8765}/dev/app-harness.html?provider=${provider}${soul}`;
  const chrome = spawn('google-chrome', ['--headless=new', '--no-sandbox', `--remote-debugging-port=${port}`, '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
    '--window-size=1000,720', `--user-data-dir=/tmp/claude-1000/chrome-${port}`, url], { stdio: 'ignore' });
  let socket, nextId = 1;
  const pending = new Map();
  const call = (method, params = {}) => new Promise((resolve, reject) => {
    const id = nextId++; pending.set(id, { resolve, reject }); socket.send(JSON.stringify({ id, method, params }));
  });
  const evaluate = async (expression) => (await call('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true })).result?.value;
  for (let i = 0; i < 60 && !socket; i++) {
    try {
      const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
      const page = targets.find((t) => t.type === 'page' && t.url.includes('app-harness'));
      if (page) { socket = new WebSocket(page.webSocketDebuggerUrl); await new Promise((r, j) => { socket.onopen = r; socket.onerror = j; }); }
    } catch { /* not up yet */ }
    if (!socket) await sleep(500);
  }
  socket.onmessage = (event) => { const m = JSON.parse(event.data); if (m.id && pending.has(m.id)) { pending.get(m.id).resolve(m.result ?? m); pending.delete(m.id); } };
  await call('Runtime.enable');

  /** A screenshot saved as `${tag}-${name}.png`, with the slime's bounding box, and the solid-pixel test at a point. */
  async function shot(name) {
    const { data } = await call('Page.captureScreenshot', { format: 'png' });
    writeFileSync(`${out}/${tag}-${name}.png`, Buffer.from(data, 'base64'));
    return evaluate(`(async () => {
      const img = new Image(); img.src = 'data:image/png;base64,${data}'; await img.decode();
      const c = document.createElement('canvas'); c.width = img.width; c.height = img.height;
      const g = c.getContext('2d'); g.drawImage(img, 0, 0);
      const px = g.getImageData(0, 0, c.width, c.height).data;
      const bg = [27, 34, 54];
      const solid = (x, y) => { const i = (y * c.width + x) * 4; return Math.abs(px[i] - bg[0]) + Math.abs(px[i + 1] - bg[1]) + Math.abs(px[i + 2] - bg[2]) > 40; };
      let x0 = 1e9, y0 = 1e9, x1 = -1, y1 = -1;
      for (let y = 0; y < c.height; y++) for (let x = 0; x < c.width; x++) if (solid(x, y)) { if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; }
      const column = (x) => { const ys = []; for (let y = y0; y <= y1; y++) if (solid(x, y)) ys.push(y); return ys; };
      const row = (y) => { const xs = []; for (let x = x0; x <= x1; x++) if (solid(x, y)) xs.push(x); return xs; };
      const top = (() => { for (let y = y0; y <= y1; y++) { const xs = row(y); if (xs.length) return [xs[xs.length >> 1], y + 8]; } })();
      const left = (() => { for (let x = x0; x <= x1; x++) { const ys = column(x); if (ys.length) return [x + 8, ys[ys.length >> 1]]; } })();
      const right = (() => { for (let x = x1; x >= x0; x--) { const ys = column(x); if (ys.length) return [x - 8, ys[ys.length >> 1]]; } })();
      return { box: [x0, y0, x1, y1], w: c.width, h: c.height, pts: { top, left, right, center: [(x0 + x1) >> 1, (y0 + y1) >> 1] } };
    })()`);
  }
  /** Is the pixel at (x, y) of the last screenshot gel? Taken from a fresh capture so it is exact. */
  async function solidAt(x, y) {
    const { data } = await call('Page.captureScreenshot', { format: 'png' });
    return evaluate(`(async () => { const img = new Image(); img.src = 'data:image/png;base64,${data}'; await img.decode();
      const c = document.createElement('canvas'); c.width = img.width; c.height = img.height; const g = c.getContext('2d'); g.drawImage(img, 0, 0);
      const d = g.getImageData(${x}, ${y}, 1, 1).data; return Math.abs(d[0] - 27) + Math.abs(d[1] - 34) + Math.abs(d[2] - 54) > 40; })()`);
  }
  /** How many pixels of gel there are in the column x, from a fresh capture. */
  async function solidColumn(x) {
    const { data } = await call('Page.captureScreenshot', { format: 'png' });
    return evaluate(`(async () => { const img = new Image(); img.src = 'data:image/png;base64,${data}'; await img.decode();
      const c = document.createElement('canvas'); c.width = img.width; c.height = img.height; const g = c.getContext('2d'); g.drawImage(img, 0, 0);
      const d = g.getImageData(${x}, 0, 1, c.height).data; let n = 0;
      for (let y = 0; y < c.height; y++) if (Math.abs(d[y * 4] - 27) + Math.abs(d[y * 4 + 1] - 34) + Math.abs(d[y * 4 + 2] - 54) > 40) n++;
      return n; })()`);
  }
  const mouse = (type, x, y, extra = {}) => call('Input.dispatchMouseEvent', { type, x, y, button: 'left', buttons: type === 'mouseReleased' ? 0 : 1, clickCount: 1, ...extra });
  const hover = (x, y) => call('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0 });
  /** Stops the slime wandering, through the same menu entry a person would use. */
  const stopRoaming = () => evaluate(`(() => { const roam = (window.__menu || []).find((i) => i.id === 'roam'); if (roam) roam.action(); return !!roam; })()`);
  return { call, evaluate, shot, solidAt, solidColumn, mouse, hover, stopRoaming, errors: () => evaluate('window.__errors'), close: () => chrome.kill() };
}
