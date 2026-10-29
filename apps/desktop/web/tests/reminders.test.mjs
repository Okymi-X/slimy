import test from 'node:test';
import assert from 'node:assert/strict';
import { whenToEpoch, describeRequest, createReminderPoller } from '../reminders.js';

const now = new Date(2026, 9, 29, 10, 0, 0);
const sec = (d) => Math.floor(d.getTime() / 1000);

test('minutes become an absolute time and bad values are refused', () => {
  assert.equal(whenToEpoch({ minutes: 20 }, now).at, sec(now) + 1200);
  assert.equal(whenToEpoch({ minutes: 0 }, now), null);
  assert.equal(whenToEpoch({ minutes: 999999 }, now), null);
  assert.equal(whenToEpoch({}, now), null);
});

test('a clock time is today when ahead and tomorrow when behind', () => {
  assert.equal(whenToEpoch({ at: '18:30' }, now).at, sec(new Date(2026, 9, 29, 18, 30)));
  assert.equal(whenToEpoch({ at: '9h15' }, now).at, sec(new Date(2026, 9, 30, 9, 15)));
  assert.equal(whenToEpoch({ at: '25:00' }, now), null);
  assert.equal(whenToEpoch({ at: 'soon' }, now), null);
});

test('a request is described by what it will do', () => {
  assert.equal(describeRequest({ server: 'slimy', name: 'files', arguments: { path: 'a/b.txt' } }), '"a/b.txt"');
  assert.match(describeRequest({ server: 'slimy', name: 'remember', arguments: { key: 'color', value: 'green' } }), /color = green/);
  assert.equal(describeRequest({ server: 'weather', name: 'get', arguments: { city: 'Paris' } }), '{"city":"Paris"}');
});

test('due reminders are announced one at a time, only when free', async () => {
  let isBusy = true; const said = [];
  const p = createReminderPoller({ invoke: async () => [{ text: 'a' }, { text: 'b' }], say: (k, v) => v.text, busy: () => isBusy, announce: (m) => said.push(m) });
  await p.poll(); p.flush();
  assert.deepEqual(said, []);
  isBusy = false; p.flush();
  assert.deepEqual(said, ['a']);
  assert.equal(p.pending, 1);
});
