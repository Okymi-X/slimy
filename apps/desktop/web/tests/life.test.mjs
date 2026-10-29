import test from 'node:test';
import assert from 'node:assert/strict';
import { createLife } from '../life.js';

function fakes({ nextQuestion = '' } = {}) {
  const calls = [];
  let ignoredId = '';
  let question = nextQuestion;
  const companion = {
    tick() {}, caress(s) { calls.push(['caress', s]); }, affection: () => 0.5, trust: () => 0.5, resentment: () => 0,
    sulking: () => false,
    ask(now, idle, returned, caressed) { calls.push(['ask', returned, caressed]); const id = question; question = ''; return id; },
    answer(id) { calls.push(['answer', id]); },
    decline(id) { calls.push(['decline', id]); },
    check_ignored() { const id = ignoredId; ignoredId = ''; return id; },
  };
  const shown = [];
  const dialogue = { show: (v) => shown.push(v), hide: () => shown.push('hide') };
  const invoked = [];
  const invoke = async (cmd, args) => { invoked.push([cmd, args]); return null; };
  const life = createLife({ companion, dialogue, invoke, language: 'fr', questionKey: (id) => `k_${id}`, random: () => 0, startAt: 0 });
  return { life, calls, shown, invoked, setIgnored: (id) => { ignoredId = id; }, setQuestion: (id) => { question = id; } };
}

const run = (life, from, to, input = {}, step = 0.5) => { for (let t = from; t <= to; t += step) life.update(t, { stroking: false, active: false, idleFor: 0, ...input }); };

test('a second of stroking is one caress', () => {
  const f = fakes();
  run(f.life, 0.5, 3, { stroking: true });
  assert.ok(f.calls.filter((c) => c[0] === 'caress').length >= 2);
});

test('a question is shown with a field and a later button', () => {
  const f = fakes({ nextQuestion: 'color' });
  run(f.life, 0.5, 5);
  const view = f.shown.find((v) => v !== 'hide');
  assert.match(view.message, /couleur/);
  assert.equal(view.field.later, 'Plus tard');
  assert.equal(f.life.asking, true);
});

test('answering stores the fact under the question key and thanks the user', async () => {
  const f = fakes({ nextQuestion: 'color' });
  run(f.life, 0.5, 5);
  assert.equal(await f.life.answer('  bleu  '), null);
  assert.deepEqual(f.invoked[0], ['soul_learn', { key: 'k_color', value: 'bleu' }]);
  assert.ok(f.calls.some((c) => c[0] === 'answer' && c[1] === 'color'));
  assert.equal(f.shown.at(-1).message, 'Noté !');
  run(f.life, 5.5, 9);
  assert.equal(f.shown.at(-1), 'hide');
});

test('an empty answer is refused politely and the question stays', async () => {
  const f = fakes({ nextQuestion: 'job' });
  run(f.life, 0.5, 5);
  assert.match(await f.life.answer('   '), /Dis-moi/);
  assert.equal(f.life.asking, true);
  assert.equal(f.invoked.length, 0);
});

test('later declines without being counted as ignored', () => {
  const f = fakes({ nextQuestion: 'hobby' });
  run(f.life, 0.5, 5);
  f.life.later();
  assert.deepEqual(f.calls.at(-1), ['decline', 'hobby']);
  assert.equal(f.life.asking, false);
});

test('a question left alone is withdrawn', () => {
  const f = fakes({ nextQuestion: 'pet' });
  run(f.life, 0.5, 5);
  f.setIgnored('pet');
  run(f.life, 5.5, 10);
  assert.equal(f.shown.at(-1), 'hide');
  assert.equal(f.life.asking, false);
});

test('coming back after a long absence is reported to the companion', () => {
  const f = fakes({ nextQuestion: 'food' });
  f.life.update(1000, { stroking: false, active: true, idleFor: 0 });
  f.life.update(1005, { stroking: false, active: true, idleFor: 0 });
  assert.equal(f.calls.find((c) => c[0] === 'ask')[1], true);
});

function withEvents(overrides = {}) {
  const events = [];
  let sulking = false;
  let ignoredId = '';
  const companion = {
    tick() {}, caress() {}, affection: () => 0.5, trust: () => 0.5, resentment: () => 0,
    sulking: () => sulking, ask: () => '', answer() {}, decline() {},
    check_ignored() { const id = ignoredId; ignoredId = ''; return id; },
    ...overrides,
  };
  const dialogue = { show() {}, hide() {} };
  const life = createLife({
    companion, dialogue, invoke: async () => null, language: 'fr', questionKey: (id) => `k_${id}`, random: () => 0, startAt: 0,
    onCaress: (s) => events.push(['caress', s]), onAnswered: () => events.push(['answered']),
    onIgnored: () => events.push(['ignored']), onCalmedAlone: () => events.push(['calmedAlone']),
  });
  return { life, events, setSulking: (v) => { sulking = v; }, setIgnored: (id) => { ignoredId = id; } };
}

test('caresses, answers and ignored questions are reported for the day', async () => {
  const f = withEvents({ ask: (() => { let given = false; return () => (given ? '' : ((given = true), 'color')); })() });
  run(f.life, 0.5, 3, { stroking: true });
  assert.ok(f.events.some((e) => e[0] === 'caress' && e[1] >= 1));
  run(f.life, 3.5, 9);
  await f.life.answer('bleu');
  assert.deepEqual(f.events.filter((e) => e[0] === 'answered'), [['answered']]);
  f.setIgnored('color');
  run(f.life, 10, 15);
  assert.deepEqual(f.events.filter((e) => e[0] === 'ignored'), [['ignored']]);
});

test('a sulk that ends by itself is reported, one ended by a caress is not', () => {
  const alone = withEvents();
  alone.setSulking(true);
  run(alone.life, 0.5, 5);
  alone.setSulking(false);
  run(alone.life, 5.5, 8);
  assert.deepEqual(alone.events.filter((e) => e[0] === 'calmedAlone'), [['calmedAlone']]);

  const soothed = withEvents();
  soothed.setSulking(true);
  run(soothed.life, 0.5, 5);
  run(soothed.life, 5.5, 8, { stroking: true });
  soothed.setSulking(false);
  run(soothed.life, 8.5, 12);
  assert.deepEqual(soothed.events.filter((e) => e[0] === 'calmedAlone'), []);
});

test('a morning journal or chat postpones new questions without losing them', () => {
  const f = fakes({ nextQuestion: 'color' });
  run(f.life, 0.5, 10, { busy: true });
  assert.equal(f.life.asking, false);
  assert.equal(f.shown.length, 0);
  run(f.life, 10.5, 15);
  assert.equal(f.life.asking, true);
});
