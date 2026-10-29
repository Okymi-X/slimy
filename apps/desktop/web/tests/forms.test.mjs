import test from 'node:test';
import assert from 'node:assert/strict';
import { formReach, profile, formForProvider, formColors, drawAccessories, POINTS } from '../forms.js';

test('each brain has its own body, the others keep the plain slime', () => {
  assert.equal(formForProvider('anthropic'), 'crab');
  assert.equal(formForProvider('openai'), 'cloud');
  assert.equal(formForProvider('deepseek'), 'whale');
  assert.equal(formForProvider('xai'), 'ufo');
  assert.equal(formForProvider('ollama'), 'llama');
  assert.equal(formForProvider('openrouter'), 'bat');
  assert.equal(formForProvider('bedrock'), 'rock');
  for (const id of ['groq', 'custom', 'nonsense', undefined]) assert.equal(formForProvider(id), 'slime');
});

test('a profile has one finite multiplier per outline point, inside the body limits', () => {
  for (const name of ['slime', 'crab', 'cloud', 'whale', 'ufo', 'llama', 'bat', 'rock', 'unknown']) {
    const p = profile(name);
    assert.equal(p.length, POINTS);
    for (const k of p) assert.ok(Number.isFinite(k) && k >= 0.3 && k <= 2, `${name}: ${k}`);
  }
  assert.ok(profile('slime').every((k) => k === 1));
});

test('the silhouettes are the ones asked for', () => {
  const crab = profile('crab'), whale = profile('whale'), cloud = profile('cloud');
  const side = 0, floor = POINTS / 4, ceiling = (3 * POINTS) / 4;
  assert.ok(Math.max(...crab.slice(0, 6)) > crab[ceiling] * 1.5, 'the crab has claws at its sides');
  assert.ok(Math.max(...crab.slice(8, 12)) > crab[floor - 1] * 1.15, 'the crab has legs');
  assert.ok(whale[Math.round((POINTS * (Math.PI * 2 - 0.78)) / (Math.PI * 2))] > 1.2, 'the whale raises its tail');
  assert.ok(cloud[floor] < cloud[ceiling], 'the cloud is flat underneath');
  assert.notDeepEqual(Array.from(crab), Array.from(whale));
});

test('brand colors exist for the forms, and the plain slime keeps its own', () => {
  for (const name of ['crab', 'cloud', 'whale', 'ufo', 'llama', 'bat', 'rock']) {
    const c = formColors(name);
    for (const key of ['top', 'mid', 'bot', 'light', 'core']) assert.equal(c[key].length, 3);
  }
  assert.equal(formColors('slime'), null);
});

test('accessories appear only once a material has really developed', () => {
  const calls = [];
  const ctx = new Proxy({}, { get: (_, k) => (k === 'createLinearGradient' ? () => ({ addColorStop() {} }) : (...a) => { calls.push(k); return undefined; }), set: () => true });
  const m = { cx: 0, minY: -60, w: 130, h: 120 };
  drawAccessories(ctx, m, { fur: 0.1, crystal: 0.2, honey: 0.39 }, [0.5, 0.5, 0.5]);
  assert.equal(calls.length, 0);
  drawAccessories(ctx, m, { fur: 0.9, crystal: 0, honey: 0 }, [0.5, 0.5, 0.5]);
  assert.ok(calls.includes('fill'));
  assert.doesNotThrow(() => drawAccessories(ctx, m, undefined, [0, 0, 0]));
});

test('each form brings its own base dome, the plain slime keeps its model', async () => {
  const { formShape, drawFormParts } = await import('../forms.js');
  const plain = [0.9, 0.5, 1.04];
  assert.deepEqual(formShape('slime', plain), plain);
  for (const name of ['crab', 'cloud', 'whale', 'ufo', 'llama', 'bat', 'rock']) assert.notDeepEqual(formShape(name, plain), plain);
  const ctx = new Proxy({}, { get: (_, k) => (k === 'createLinearGradient' ? () => ({ addColorStop() {} }) : () => undefined), set: () => true });
  const palette = { top: [1, 1, 1], mid: [0.5, 0.5, 0.5], bot: [0, 0, 0], light: [1, 1, 1], core: [0, 0, 0] };
  for (const name of ['crab', 'cloud', 'whale', 'slime', 'nope']) assert.doesNotThrow(() => drawFormParts(ctx, name, { cx: 0, minX: -60, maxX: 60, minY: -50, maxY: 40, w: 120, h: 90 }, palette, 1));
});

test('limbs are polygons of finite points in the body frame, and only the forms with limbs have any', async () => {
  const { limbPolygons } = await import('../forms.js');
  const m = { cx: 0, minX: -70, maxX: 70, minY: -50, maxY: 40, w: 140, h: 90 };
  assert.deepEqual(limbPolygons('slime', m, 0), []);
  const counts = { crab: 6, whale: 3, cloud: 3, ufo: 2, llama: 5, bat: 4, rock: 3 };
  for (const [name, count] of Object.entries(counts)) {
    const polys = limbPolygons(name, m, 1.3);
    assert.equal(polys.length, count, name);
    for (const poly of polys) {
      assert.ok(poly.length >= 4);
      for (const [x, y] of poly) assert.ok(Number.isFinite(x) && Number.isFinite(y));
    }
    assert.ok(polys.reduce((n, p) => n + p.length, 0) + 56 <= 192, `${name} fits the shader`);
  }
});

test('a point on a limb is told apart from the body, and a grab lands inside the body', async () => {
  const { insidePolygon, insideFlat, bodyPointNear } = await import('../forms.js');
  const square = [[0, 0], [10, 0], [10, 10], [0, 10]];
  assert.equal(insidePolygon(square, 5, 5), true);
  assert.equal(insidePolygon(square, 15, 5), false);
  const body = [];
  for (let i = 0; i < 56; i++) { const a = (i / 56) * Math.PI * 2; body.push(100 + Math.cos(a) * 60, 100 + Math.sin(a) * 40); }
  assert.equal(insideFlat(body, 100, 100), true);
  assert.equal(insideFlat(body, 190, 100), false);
  const near = bodyPointNear(body, 190, 100, 12);
  assert.ok(insideFlat(body, near.x, near.y), 'the proxy is inside the body');
  assert.ok(near.x > 130, `it is on the side of the limb: ${near.x}`);
});

test('limbs swing against the push of the body, and a rest push leaves them where they were', async () => {
  const { limbPolygons } = await import('../forms.js');
  const m = { cx: 0, minX: -70, maxX: 70, minY: -50, maxY: 40, w: 140, h: 90 };
  const centre = (polys) => polys.slice(0, 3).flat().reduce((sum, p) => sum + p[0], 0) / polys.slice(0, 3).flat().length;
  const still = centre(limbPolygons('whale', m, 0, { x: 0, y: 0 }));
  const swung = centre(limbPolygons('whale', m, 0, { x: -0.4, y: 0 }));
  assert.ok(swung < still - 3, `the tail swings left: ${swung} vs ${still}`);
  const claw = (p) => p[0].reduce((s, q) => s + q[0], 0) / p[0].length;
  assert.ok(claw(limbPolygons('crab', m, 0, { x: -0.5, y: 0 })) < claw(limbPolygons('crab', m, 0, { x: 0, y: 0 })), 'claws lag behind');
});

test('a dark gel gets a light face, every other form keeps the model ink', async () => {
  const { formInk, formColors } = await import('../forms.js');
  assert.ok(/^#[ef]/i.test(formInk('ufo')), 'light ink on the dark saucer slime');
  for (const name of ['slime', 'crab', 'cloud', 'whale', 'llama', 'bat', 'rock']) assert.equal(formInk(name), null, name);
  // Whatever the form, the face stays readable: dark ink only on gel that is not dark.
  const lum = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;
  for (const name of ['crab', 'cloud', 'whale', 'ufo', 'llama', 'bat', 'rock']) {
    const body = lum(formColors(name).mid), ink = formInk(name);
    if (ink) assert.ok(body < 0.4, `${name}: light ink on a dark body`); else assert.ok(body > 0.2, `${name}: dark ink needs a body brighter than 0.2, got ${body.toFixed(2)}`);
  }
});

test('a wide form reports a reach beyond one radius, a plain or tall one about one', () => {
  assert.ok(formReach('crab') > 1.3, 'claws widen the crab');
  assert.ok(formReach('bat') >= 1, 'a bat is at least as wide as the base');
  assert.equal(formReach('slime'), 1);
  assert.ok(formReach('llama') < 1.3);
  assert.equal(formReach('nope'), 1);
});

test('horns, wings and leaves each appear with their own material and not before', () => {
  const m = { cx: 0, minY: -60, w: 130, h: 120 };
  const count = (layers) => {
    const calls = [];
    const ctx = new Proxy({}, { get: (_, k) => (k === 'createLinearGradient' ? () => ({ addColorStop() {} }) : (...a) => { calls.push(k); }), set: () => true });
    drawAccessories(ctx, m, layers, [0.5, 0.5, 0.5]);
    return calls.filter((k) => k === 'fill').length;
  };
  assert.equal(count({ lava: 0.3, stars: 0.3, stripes: 0.3 }), 0);
  assert.equal(count({ lava: 0.9 }), 2, 'two horns');
  assert.equal(count({ stars: 0.9 }), 2, 'two wings');
  assert.equal(count({ stripes: 0.9 }), 2, 'two leaves');
  assert.equal(count({ lava: 0.9, stars: 0.9, stripes: 0.9 }), 6);
});
