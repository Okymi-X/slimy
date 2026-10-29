// The slime adapts to the screen on the real page (dev/app-harness.html, backend faked): its size follows the screen's
// `size_scale`, and when the play area shrinks while it runs (a screen unplugged) it ends up inside the new area, without
// reloading. Needs `python3 -m http.server 8765` in apps/desktop/web.
import { launch, sleep } from './cdp.mjs';

async function widthAt(size, port) {
  const page = await launch({ provider: 'groq', port, tag: `display-${size}`, soul: `&size=${size}` });
  await sleep(6000);
  const shot = await page.shot('size');
  page.close();
  return shot.box[2] - shot.box[0];
}

(async () => {
  const checks = [];
  const check = (name, ok) => { checks.push(ok); console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}`); };
  const small = await widthAt(0.8, '9480'), normal = await widthAt(1, '9481'), big = await widthAt(1.3, '9482');
  console.log(`slime width: small screen ${small}px, normal ${normal}px, big ${big}px`);
  check('a small screen gets a smaller slime and a big one a bigger slime', small < normal && normal < big);
  check('the sizes follow the scale (within 20 %)', Math.abs(big / normal - 1.3) < 0.2 && Math.abs(small / normal - 0.8) < 0.2);

  const page = await launch({ provider: 'groq', port: '9483', tag: 'display-live' });
  await sleep(6000);
  const first = await page.shot('before');
  const reloads = await page.evaluate(`(window.__reloads = 0, performance.getEntriesByType('navigation').length)`);
  // the world shrinks to the top-left 640 x 360 of the page
  await page.evaluate(`window.__emit('world-changed', { left: 0, top: 0, right: 640, bottom: 360, scale_factor: 1, size_scale: 1, index: 0, count: 1, preference: 'primary' })`);
  await sleep(5000);
  const after = await page.shot('after');
  check('the slime was drawn before the change', first.box[2] > 0);
  check('after the play area shrinks it lives inside it', after.box[2] <= 640 + 4 && after.box[3] <= 360 + 4);
  check('it did not reload', await page.evaluate(`performance.getEntriesByType('navigation').length`) === reloads);
  const errors = await page.errors();
  check('no page errors', errors.length === 0);
  if (errors.length) console.log(JSON.stringify(errors));
  console.log(`box before ${first.box}, after ${after.box}`);
  page.close();
  console.log(checks.every(Boolean) ? 'RESULT PASS' : 'RESULT FAIL');
  process.exit(checks.every(Boolean) ? 0 : 1);
})();
