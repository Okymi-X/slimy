import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { applyLook, createDayCounter, createNightJournal, createThrowDetector, isNightHour, journalLine, localDay } from '../genetics.js';
import { normalizeModel } from '../model.js';

const base = JSON.parse(readFileSync(new URL('../models/base-slime.json', import.meta.url), 'utf8'));

const look = {
  radius: 50, base_width: 1.1, breathing_speed: 2.0,
  colors: { top: [1, 0.8, 0.8], mid: [1, 0.5, 0.5], bot: [0.8, 0.2, 0.2], light: [1, 0.9, 0.9], core: [0.7, 0.1, 0.1] },
  opacity: 0.7, edge_opacity: 0.2, glow: 0.4, core_strength: 0.5, bubbles: 8,
  blush: false, eye_width: 0.06, eye_separation: 0.2, eye_aspect: 1.6,
};

test('a look is applied to the model and the rest of the model is kept', () => {
  const model = applyLook(base, look);
  assert.equal(model.body.radius, 50);
  assert.equal(model.body.shape.baseWidth, 1.1);
  assert.equal(model.body.shape.top, base.body.shape.top);
  assert.equal(model.body.breathing.speed, 2.0);
  assert.deepEqual(model.material.colors.mid, [1, 0.5, 0.5]);
  assert.equal(model.material.opacity, 0.7);
  assert.equal(model.material.gloss, base.material.gloss);
  assert.equal(model.bubbles.count, 8);
  assert.equal(model.face.blush, false);
  assert.equal(model.face.eyes.width, 0.06);
  assert.equal(model.face.eyes.color, base.face.eyes.color);
});

test('no look gives the plain base model', () => {
  assert.deepEqual(applyLook(base, null), normalizeModel(base));
});

test('applying a look never changes the base model it came from', () => {
  const before = JSON.stringify(base);
  applyLook(base, look);
  assert.equal(JSON.stringify(base), before);
});

test('a hostile look is clamped by the model, never an error', () => {
  const hostile = { ...look, radius: 1e9, opacity: -4, bubbles: 999, eye_width: NaN, colors: { ...look.colors, mid: [9, -9, NaN] } };
  const model = applyLook(base, hostile);
  assert.ok(model.body.radius <= 160);
  assert.ok(model.material.opacity >= 0.2);
  assert.ok(model.bubbles.count <= 12);
  assert.ok(Number.isFinite(model.face.eyes.width));
  assert.ok(model.material.colors.mid.every((v) => v >= 0 && v <= 1));
});

test('the journal names a gene, a mutation, or a quiet night', () => {
  assert.match(journalLine('fr', { mutation: null, headline: 'cheeks' }), /mes joues/);
  assert.match(journalLine('en', { mutation: null, headline: 'cheeks' }), /my cheeks/);
  assert.match(journalLine('fr', { mutation: 'stars', headline: 'cheeks' }), /étoiles/);
  assert.equal(journalLine('fr', { mutation: null, headline: null }), "J'ai bien dormi.");
  assert.equal(journalLine('fr', null), null);
});

test('every gene the core can name has a sentence in both languages', () => {
  const ids = ['skin_hue', 'saturation', 'lightness', 'transparency', 'core_shift', 'highlight_position', 'glow', 'viscosity', 'size', 'flatness', 'bounce', 'outline', 'eye_size', 'eye_spacing', 'eye_shape', 'cheeks', 'core_vividness', 'bubbles', 'stripes', 'honey', 'crystal', 'fur', 'lava', 'stars', 'accessory_growth'];
  for (const id of ids) {
    for (const language of ['fr', 'en']) {
      const line = journalLine(language, { mutation: null, headline: id });
      assert.ok(line && !line.includes('undefined') && !line.includes('gene_'), `${language} ${id}: ${line}`);
    }
  }
});

test('the local day changes at local midnight, not at UTC midnight', () => {
  const a = localDay(new Date(2026, 9, 25, 23, 59));
  const b = localDay(new Date(2026, 9, 26, 0, 1));
  assert.equal(b - a, 1);
});

test('night hours run from 22 to 5', () => {
  assert.deepEqual([21, 22, 23, 0, 4, 5, 12].map(isNightHour), [false, true, true, true, true, false, false]);
});

test('the counter reports once a minute and only when something happened', async () => {
  const calls = [];
  const counter = createDayCounter({ invoke: async (name, args) => { calls.push([name, args]); }, today: () => 20000, hour: () => 12 });
  counter.tick(0, 1, 100);
  counter.tick(61, 1, 100);
  await counter.flush();
  assert.equal(calls.length, 0, 'an idle minute is not reported');
  counter.caress(5);
  counter.thrown();
  counter.tick(130, 1, 0);
  await counter.flush();
  assert.equal(calls.length, 1);
  assert.equal(calls[0][0], 'genome_day');
  assert.equal(calls[0][1].today, 20000);
  assert.equal(calls[0][1].delta.caress_seconds, 5);
  assert.equal(calls[0][1].delta.throws, 1);
  assert.equal(calls[0][1].delta.night_seconds, 0);
  assert.deepEqual(counter.pending, { caress_seconds: 0, throws: 0, answered: 0, ignored: 0, messages: 0, active_seconds: 0, night_seconds: 0, calmed_alone: 0 });
});

test('time with the user counts as night use only at night', () => {
  const counter = createDayCounter({ invoke: async () => {}, hour: () => 23 });
  counter.tick(0, 2, 5);
  assert.equal(counter.pending.active_seconds, 2);
  assert.equal(counter.pending.night_seconds, 2);
  counter.tick(1, 2, 500);
  assert.equal(counter.pending.active_seconds, 2, 'away from the keyboard is not time together');
});

test('a failed report is kept and sent again', async () => {
  let fail = true;
  const sent = [];
  const counter = createDayCounter({ invoke: async (_, args) => { if (fail) throw new Error('io'); sent.push(args.delta); } });
  counter.answered();
  await counter.flush();
  assert.equal(counter.pending.answered, 1);
  fail = false;
  await counter.flush();
  assert.equal(sent[0].answered, 1);
  assert.equal(counter.pending.answered, 0);
});

test('a throw is a fast release, a slow put down is not', () => {
  const detect = createThrowDetector();
  assert.equal(detect(true, 0), false);
  assert.equal(detect(false, 100), false, 'released slowly');
  assert.equal(detect(true, 0), false);
  assert.equal(detect(false, 900), true);
  assert.equal(detect(false, 900), false, 'only on the release frame');
});

test('reports keep their original day across midnight and failed retries', async () => {
  let date = 20000, fail = true;
  const calls = [];
  const counter = createDayCounter({ today: () => date, invoke: async (_, args) => {
    if (fail) throw new Error('offline');
    calls.push(args);
  } });
  counter.answered();
  await counter.flush();
  date++;
  counter.message();
  fail = false;
  await counter.flush();
  assert.deepEqual(calls.map((c) => [c.today, c.delta.answered, c.delta.messages]), [[20000, 1, 0], [20001, 0, 1]]);
});

test('a flush requested during an active report waits for the remaining events', async () => {
  let release;
  const calls = [];
  const counter = createDayCounter({ invoke: async (_, args) => {
    calls.push(args);
    if (calls.length === 1) await new Promise((resolve) => { release = resolve; });
  } });
  counter.answered();
  const first = counter.flush();
  counter.message();
  let finished = false;
  const closing = counter.flush().then(() => { finished = true; });
  await Promise.resolve();
  assert.equal(finished, false);
  release();
  await Promise.all([first, closing]);
  assert.equal(calls.length, 2);
  assert.equal(calls[1].delta.messages, 1);
});

test('a quiet midnight still closes the day and delivers its night report', async () => {
  let date = 20000;
  const nights = [];
  const counter = createDayCounter({ today: () => date, onNight: (night) => nights.push(night), invoke: async () => ({ night: 1 }) });
  counter.tick(0, 0, 100);
  date++;
  counter.tick(1, 0, 100);
  await counter.flush();
  assert.deepEqual(nights, [{ night: 1 }]);
});

test('the night journal waits for the current conversation and does not hide a new one', () => {
  const dialogue = { visible: true, message: 'chat', show({ message }) { this.message = message; this.visible = true; }, hide() { this.visible = false; } };
  const journal = createNightJournal({ dialogue, language: 'en' });
  journal.enqueue({ headline: 'cheeks' });
  journal.tick(0, true);
  assert.equal(dialogue.message, 'chat');
  dialogue.hide();
  journal.tick(2, false);
  assert.match(dialogue.message, /my cheeks/);
  assert.equal(journal.active, true);
  dialogue.show({ message: 'new chat' });
  journal.tick(3, true);
  journal.tick(20, true);
  assert.equal(dialogue.visible, true);
  assert.equal(dialogue.message, 'new chat');
});

test('the night journal disappears after seven seconds', () => {
  const dialogue = { visible: false, show() { this.visible = true; }, hide() { this.visible = false; } };
  const journal = createNightJournal({ dialogue, language: 'en' });
  journal.enqueue({ headline: null });
  journal.tick(0, false);
  journal.tick(6, false);
  assert.equal(dialogue.visible, true);
  journal.tick(7, false);
  assert.equal(dialogue.visible, false);
});

test('material genes reach the renderer without changing the base model', () => {
  const layers = { stripes: 0.2, honey: 0.4, crystal: 0.6, fur: 0.8, lava: 1, stars: 0.3 };
  const before = structuredClone(base);
  assert.deepEqual(applyLook(base, { ...look, layers }).material.layers, layers);
  assert.deepEqual(base, before);
  assert.ok(Object.values(normalizeModel(base).material.layers).every((value) => value === 0));
});

test('invalid material intensities are bounded independently', () => {
  const layers = { stripes: -2, honey: 4, crystal: NaN, fur: Infinity, lava: '1', stars: 0.7, unknown: 1 };
  assert.deepEqual(applyLook(base, { ...look, layers }).material.layers,
    { stripes: 0, honey: 1, crystal: 0, fur: 0, lava: 0, stars: 0.7 });
});
