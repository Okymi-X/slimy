import test from 'node:test';
import assert from 'node:assert/strict';
import { createLimbJelly } from '../limbjelly.js';

const body = { minX: -60, maxX: 60, minY: -40, maxY: 40, w: 120 };
const limb = () => [[[50, 0], [90, 0], [90, 10], [50, 10]]];
const mean = (poly, k) => poly.reduce((s, p) => s + p[k], 0) / poly.length;

test('an accelerating body makes the tip of a limb lag more than its root', () => {
  const jelly = createLimbJelly();
  let polys = limb();
  for (let i = 0; i < 30; i++) { polys = limb(); jelly.apply(polys, { body, acc: { x: 3000, y: 0 }, pointer: null, dt: 1 / 60 }); }
  const [root, , tip] = [polys[0][0], polys[0][1], polys[0][2]];
  assert.ok(root[0] - 50 < 0, 'the root lags too, a little');
  assert.ok(tip[0] - 90 < root[0] - 50, `tip ${tip[0] - 90} lags more than root ${root[0] - 50}`);
});

test('a pointer pushes the nearby gel away and the limb springs back when it leaves', () => {
  const jelly = createLimbJelly();
  let polys;
  for (let i = 0; i < 40; i++) { polys = limb(); jelly.apply(polys, { body, acc: { x: 0, y: 0 }, pointer: { x: 100, y: 5 }, dt: 1 / 60 }); }
  assert.ok(mean(polys[0], 0) < 70 - 1, `dented away from the pointer: ${mean(polys[0], 0)}`);
  for (let i = 0; i < 200; i++) { polys = limb(); jelly.apply(polys, { body, acc: { x: 0, y: 0 }, pointer: null, dt: 1 / 60 }); }
  assert.ok(Math.abs(mean(polys[0], 0) - 70) < 0.5, `back to rest: ${mean(polys[0], 0)}`);
});

test('offsets stay bounded whatever the push, and nothing moves at rest', () => {
  const jelly = createLimbJelly();
  let polys;
  for (let i = 0; i < 60; i++) { polys = limb(); jelly.apply(polys, { body, acc: { x: 9e4, y: -9e4 }, pointer: { x: 95, y: 5 }, dt: 0.5 }); }
  for (const [x, y] of polys[0]) assert.ok(Number.isFinite(x) && Math.hypot(x - 70, y - 5) < 60);
  const calm = createLimbJelly();
  polys = limb();
  calm.apply(polys, { body, acc: { x: 0, y: 0 }, pointer: null, dt: 1 / 60 });
  assert.deepEqual(polys, limb());
});

test('a changed number of polygons or vertices restarts the soft layer instead of failing', () => {
  const jelly = createLimbJelly();
  jelly.apply(limb(), { body, acc: { x: 100, y: 0 }, pointer: null, dt: 1 / 60 });
  const other = [[[0, 0], [5, 0], [5, 5]], [[0, 0], [1, 1], [2, 0], [1, -1]]];
  assert.doesNotThrow(() => jelly.apply(other, { body, acc: { x: 100, y: 0 }, pointer: null, dt: 1 / 60 }));
});

test('a held limb follows the pointer at the held point, stretches toward it, and the root stays near the body', () => {
  const jelly = createLimbJelly();
  let polys;
  for (let i = 0; i < 90; i++) {
    polys = limb();
    jelly.apply(polys, { body, acc: { x: 0, y: 0 }, pointer: null, dt: 1 / 60, hold: { poly: 0, vertex: 1, to: [140, -30] } });
  }
  const [root, tip] = [polys[0][0], polys[0][1]];
  assert.ok(Math.hypot(tip[0] - 140, tip[1] + 30) < 12, `the held point is under the pointer: ${tip}`);
  assert.ok(tip[1] < -20, 'it stretched upward');
  assert.ok(Math.hypot(root[0] - 50, root[1] - 0) < Math.hypot(tip[0] - 90, tip[1] - 0), 'the root moved less than the tip');
});

test('letting go of a held limb brings it back, and the stretch is bounded', () => {
  const jelly = createLimbJelly();
  let polys;
  for (let i = 0; i < 90; i++) { polys = limb(); jelly.apply(polys, { body, acc: { x: 0, y: 0 }, pointer: null, dt: 1 / 60, hold: { poly: 0, vertex: 1, to: [900, -900] } }); }
  polys[0].forEach(([x, y], i) => { if (i !== 1) assert.ok(Math.hypot(x - 70, y - 5) < 60 + 1.4 * body.w, `neighbour ${i} is bounded`); });
  assert.ok(Math.hypot(polys[0][1][0] - 900, polys[0][1][1] + 900) < 1e-6, 'the held vertex itself stays on the pointer');
  for (let i = 0; i < 240; i++) { polys = limb(); jelly.apply(polys, { body, acc: { x: 0, y: 0 }, pointer: null, dt: 1 / 60 }); }
  assert.ok(Math.abs(mean(polys[0], 0) - 70) < 0.5 && Math.abs(mean(polys[0], 1) - 5) < 0.5, 'back to rest');
  const bad = createLimbJelly();
  assert.doesNotThrow(() => bad.apply(limb(), { body, acc: { x: 0, y: 0 }, pointer: null, dt: 1 / 60, hold: { poly: 9, vertex: 99, to: [0, 0] } }));
});

import { pickVertex } from '../limbjelly.js';

test('the held vertex sits exactly on the pointer, with no permanent gap', () => {
  const jelly = createLimbJelly();
  let polys;
  for (let i = 0; i < 40; i++) {
    polys = limb();
    jelly.apply(polys, { body, acc: { x: 0, y: 0 }, pointer: null, dt: 1 / 60, hold: { poly: 0, vertex: 1, to: [140, -30] } });
  }
  assert.ok(Math.hypot(polys[0][1][0] - 140, polys[0][1][1] + 30) < 1e-6, `${polys[0][1]}`);
  assert.ok(polys[0][2][0] > 90, 'a neighbour follows it along');
});

test('the motion does not depend on the frame rate', () => {
  const run = (frames, dt) => {
    const jelly = createLimbJelly();
    let polys;
    for (let i = 0; i < frames; i++) polys = (() => { const p = limb(); jelly.apply(p, { body, acc: { x: 2500, y: 0 }, pointer: null, dt }); return p; })();
    return polys[0][1];
  };
  const smooth = run(30, 1 / 60), coarse = run(5, 0.1);
  assert.ok(Math.abs(smooth[0] - coarse[0]) < 1.2 && Math.abs(smooth[1] - coarse[1]) < 1.2, `${smooth} vs ${coarse}`);
});

test('picking a vertex uses the displayed polygons, so a deformed limb picks the vertex under the cursor', () => {
  const displayed = [[[100, 0], [60, 5], [70, 30]], [[0, 0], [1, 1], [2, 0]]];
  assert.deepEqual(pickVertex(displayed, 99, 2), { poly: 0, vertex: 0, d: Math.hypot(1, 2) });
  assert.equal(pickVertex([], 0, 0), null);
});

test('letting go of a stretched limb does not snap it back: it relaxes over a moment', () => {
  const jelly = createLimbJelly();
  let polys;
  for (let i = 0; i < 40; i++) { polys = limb(); jelly.apply(polys, { body, acc: { x: 0, y: 0 }, pointer: null, dt: 1 / 60, hold: { poly: 0, vertex: 1, to: [140, -30] } }); }
  const held = polys[0][1].slice();
  polys = limb();
  jelly.apply(polys, { body, acc: { x: 0, y: 0 }, pointer: null, dt: 0 });
  assert.ok(Math.hypot(polys[0][1][0] - held[0], polys[0][1][1] - held[1]) < 1e-6, 'a zero step changes nothing');
  polys = limb();
  jelly.apply(polys, { body, acc: { x: 0, y: 0 }, pointer: null, dt: 1 / 60 });
  assert.ok(Math.hypot(polys[0][1][0] - held[0], polys[0][1][1] - held[1]) < 14, `one frame later it is still near: ${polys[0][1]}`);
  for (let i = 0; i < 120; i++) { polys = limb(); jelly.apply(polys, { body, acc: { x: 0, y: 0 }, pointer: null, dt: 1 / 60 }); }
  assert.ok(Math.hypot(polys[0][1][0] - 90, polys[0][1][1]) < 1, 'and it ends at rest');
});

test('a negative or zero time step is harmless', () => {
  const jelly = createLimbJelly();
  for (const dt of [0, -1, NaN]) {
    const polys = limb();
    assert.doesNotThrow(() => jelly.apply(polys, { body, acc: { x: 100, y: 0 }, pointer: null, dt }));
    for (const [x, y] of polys[0]) assert.ok(Number.isFinite(x) && Number.isFinite(y));
  }
});

test('the whole elapsed time is used up to 0.25 s, so a 6 fps page moves like a 60 fps one', () => {
  const at = (frames, dt) => { const jelly = createLimbJelly(); let p; for (let i = 0; i < frames; i++) { p = limb(); jelly.apply(p, { body, acc: { x: 2500, y: 0 }, pointer: null, dt }); } return p[0][1][0]; };
  assert.ok(Math.abs(at(6, 1 / 6) - at(60, 1 / 60)) < 2, `${at(6, 1 / 6)} vs ${at(60, 1 / 60)}`);
});
