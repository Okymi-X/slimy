// Grabs the slime at an extreme point of its silhouette (a tail, a claw, a wing), drags it up, lets go, and reports.
// Drives the real page (dev/app-harness.html) with real mouse events through cdp.mjs.
// Usage: node interact-test.mjs <provider> <top|left|right|center> [port]
import { launch, sleep } from './cdp.mjs';

const [provider = 'deepseek', mode = 'top', port = '9333'] = process.argv.slice(2);

(async () => {
  const page = await launch({ provider, port, tag: `grab-${provider}-${mode}` });
  await sleep(7000); // the page starts, the soul loads, the form morphs
  await page.stopRoaming();
  await sleep(3500);
  const before = await page.shot('0-before');
  const [gx, gy] = before.pts[mode];
  console.log(`${provider}/${mode}: slime box ${before.box}, grab at ${gx},${gy}, page errors ${JSON.stringify(await page.errors())}`);
  await page.hover(gx, gy);
  await sleep(300);
  await page.mouse('mousePressed', gx, gy);
  const dx = mode === 'left' ? 60 : -60, dy = -170, steps = 28;
  for (let i = 1; i <= steps; i++) { await page.mouse('mouseMoved', gx + (dx * i) / steps, gy + (dy * i) / steps); await sleep(40); }
  await sleep(300);
  const during = await page.shot('1-during');
  await page.mouse('mouseReleased', gx + dx, gy + dy);
  await sleep(3800);
  const after = await page.shot('3-after');
  const lift = before.box[1] - during.box[1], floorHeld = Math.abs(after.box[3] - before.box[3]) < 4;
  console.log(`  before top ${before.box[1]} bottom ${before.box[3]}; during top ${during.box[1]} (lifted ${lift}px); after top ${after.box[1]} bottom ${after.box[3]}`);
  const errors = await page.errors();
  const ok = lift > 80 && floorHeld && !errors.length;
  console.log(`  RESULT ${ok ? 'PASS' : 'FAIL'}: lifted ${lift}px while held, back on the floor after release: ${floorHeld}, page errors: ${errors.length}`);
  page.close();
  process.exit(ok ? 0 : 1);
})().catch((e) => { console.error('test crashed:', e); process.exit(2); });
