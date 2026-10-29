// Counts the images drawn each second while the slime lives on its own (no mouse), to compare frame-rate policies.
// Usage: node fps-test.mjs [seconds]. Needs `python3 -m http.server 8765` in apps/desktop/web.
import { launch, sleep } from './cdp.mjs';

const seconds = Number(process.argv[2] ?? 30);
(async () => {
  const page = await launch({ provider: 'groq', port: '9450', tag: 'fps' });
  await sleep(4000);
  await page.evaluate(`(() => { window.__draws = 0; const f = WebGL2RenderingContext.prototype.drawArrays; WebGL2RenderingContext.prototype.drawArrays = function (...a) { window.__draws++; return f.apply(this, a); }; })()`);
  const perSecond = [];
  let last = 0;
  for (let i = 0; i < seconds; i++) {
    await sleep(1000);
    const now = await page.evaluate('window.__draws');
    perSecond.push(now - last); last = now;
  }
  console.log(`draws per second: ${perSecond.join(' ')}`);
  console.log(`mean ${(perSecond.reduce((a, b) => a + b, 0) / seconds).toFixed(1)}, max ${Math.max(...perSecond)}, page errors ${JSON.stringify(await page.errors())}`);
  page.close(); process.exit(0);
})();
