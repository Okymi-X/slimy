// The slime catches a cursor that rests on the floor near it, with its whole body: it hops over, squashes flat on the
// cursor, holds it, and backs off. Drives the real page (dev/app-harness.html) with real mouse events (the cursor is
// moved once and then left still). A cursor high in the air must not start anything.
// Usage: node catch-test.mjs <provider> <dx> <height above the floor> [port]
import { launch, sleep } from './cdp.mjs';

const [provider = 'groq', dxArg = '70', heightArg = '20', port = '9360'] = process.argv.slice(2);
const dx = Number(dxArg), height = Number(heightArg), lowCursor = height >= 4 && height <= 48;

(async () => {
  const page = await launch({ provider, port, tag: `catch-${provider}-${dxArg}-${heightArg}` });
  await sleep(7000);
  // Let it settle: two captures with the same box (it may be mid-wander).
  let base;
  for (let i = 0; i < 40; i++) {
    const a = await page.shot('a'); await sleep(600); const b = await page.shot('b');
    // Still, and on the floor (not clinging to a wall, not in the air): a wide, low box that touches the bottom.
    const onFloor = b.box[3] >= b.h - 8 && b.box[3] - b.box[1] <= 110 && b.box[0] > 40 && b.box[2] < b.w - 40;
    if (onFloor && a.box.every((v, k) => Math.abs(v - b.box[k]) <= 2)) { base = b; break; }
  }
  if (!base) { console.log('FAIL: the slime never held still'); page.close(); process.exit(1); }
  const floorY = base.box[3], centreX = (base.box[0] + base.box[2]) / 2;
  const cursor = [Math.round(centreX + dx), floorY - height];
  console.log(`${provider}: slime box ${base.box}, cursor ${cursor} (${dx}px aside, ${height}px above the floor)`);
  await page.hover(cursor[0], cursor[1]); // moved once, then still
  const samples = [];
  for (let i = 0; i < 40; i++) {
    await sleep(250);
    const s = await page.shot(`t${String(i).padStart(2, '0')}`);
    const covered = await page.solidAt(cursor[0], cursor[1]);
    const column = covered ? await page.solidColumn(cursor[0]) : 0;
    samples.push({ t: (i + 1) * 0.25, box: s.box, covered, column });
  }
  const width = (b) => b[2] - b[0], high = (b) => b[3] - b[1];
  const held = samples.filter((s) => s.covered);
  const thickest = Math.max(0, ...held.map((s) => s.column));
  const widest = Math.max(...samples.map((s) => width(s.box))), lowest = Math.min(...samples.map((s) => high(s.box)));
  const last = samples[samples.length - 1];
  const errors = await page.errors();
  console.log(`  covered in ${held.length} of ${samples.length} samples (first at ${held[0]?.t ?? '-'} s, last at ${held[held.length - 1]?.t ?? '-'} s); gel over the cursor up to ${thickest}px`);
  console.log(`  widest ${widest}px (rest ${width(base.box)}), lowest ${lowest}px (rest ${high(base.box)}); at the end: covered ${last.covered}, box ${last.box}; page errors ${JSON.stringify(errors)}`);
  let ok;
  if (lowCursor) {
    ok = held.length >= 3 && thickest >= 45 && widest >= width(base.box) * 1.08 && lowest <= high(base.box) * 0.95 && !last.covered && !errors.length;
  } else {
    ok = held.length === 0 && !errors.length;
  }
  console.log(`  RESULT ${ok ? 'PASS' : 'FAIL'}${lowCursor ? '' : ' (a cursor out of the strip must not be caught)'}`);
  page.close();
  process.exit(ok ? 0 : 1);
})().catch((e) => { console.error('test crashed:', e); process.exit(2); });
