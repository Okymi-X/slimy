import test from 'node:test';
import assert from 'node:assert/strict';
import { todaysReminders, briefingMessage, createBriefing } from '../briefing.js';

const date = new Date(2026, 9, 29, 8, 0, 0);
const at = (h, m = 0) => Math.floor(new Date(2026, 9, 29, h, m).getTime() / 1000);
const say = (key, v = {}) => `${key}${Object.values(v).join('/')}`;

test('only today and daily reminders are kept, earliest first', () => {
  const list = [{ at: at(18), text: 'late' }, { at: at(9, 5), text: 'early' }, { at: at(9) + 86400, text: 'tomorrow' }, { at: at(7) - 86400, text: 'daily', every_day: true }];
  assert.deepEqual(todaysReminders(list, date).map((r) => r.text), ['daily', 'early', 'late']);
  assert.equal(todaysReminders(list, date)[1].time, '09:05');
  assert.deepEqual(todaysReminders(null, date), []);
});

test('nothing to say gives no message', () => {
  assert.equal(briefingMessage({ reminders: [], facts: 0, say }), null);
  assert.match(briefingMessage({ reminders: [], facts: 3, say }), /briefingFacts3/);
});

test('it speaks once per day, only when free, and not again after a restart the same day', async () => {
  const store = new Map(); const storage = { getItem: (k) => store.get(k) ?? null, setItem: (k, v) => store.set(k, v) };
  let isBusy = true; const said = [];
  const invoke = async (cmd) => (cmd === 'reminder_list' ? [{ at: at(10), text: 'call', every_day: false }] : [{}, {}]);
  const make = () => createBriefing({ invoke, say, busy: () => isBusy, announce: (m) => said.push(m), storage, now: () => date });
  const tick = make();
  await tick(); assert.equal(said.length, 0);
  isBusy = false; await tick(); await tick();
  assert.equal(said.length, 1);
  await make()(); assert.equal(said.length, 1);
});
