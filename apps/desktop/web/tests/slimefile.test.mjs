import test from 'node:test';
import assert from 'node:assert/strict';
import { isPlainSlimeName, readPreview, createSlimeFile } from '../slimefile.js';

test('only bare .slime names pass', () => {
  assert.ok(isPlainSlimeName('slimy-1700000000.slime'));
  for (const bad of ['../a.slime', 'a/b.slime', 'a\\b.slime', '.slime', 'a.json', '..slime', 'a..b.slime', '', null, 5]) {
    assert.equal(isPlainSlimeName(bad), false, String(bad));
  }
});

test('a preview needs a safe name, a name and counts', () => {
  assert.deepEqual(readPreview({ file: 'a.slime', slime_name: 'Mochi', nights: 3, facts: 2 }), { file: 'a.slime', name: 'Mochi', nights: 3, facts: 2 });
  assert.equal(readPreview({ file: '../a.slime', slime_name: 'M', nights: 1, facts: 1 }), null);
  assert.equal(readPreview({ file: 'a.slime', slime_name: 'M', nights: -1, facts: 1 }), null);
  assert.equal(readPreview({ file: 'a.slime', slime_name: 'M', nights: 1.5, facts: 1 }), null);
  assert.equal(readPreview(null), null);
});

function setup(responses) {
  const calls = [];
  const shown = [];
  const invoke = async (cmd, args) => {
    calls.push([cmd, args]);
    const r = responses[cmd];
    if (r instanceof Error) throw r.message;
    return r;
  };
  const dialogue = { show: (v) => shown.push(v) };
  let applied = 0;
  const file = createSlimeFile({ invoke, dialogue, language: 'en', onApplied: () => { applied += 1; } });
  return { file, calls, shown, applied: () => applied };
}

test('nothing is applied until Yes, and No applies nothing', async () => {
  const preview = { file: 'a.slime', slime_name: 'Mochi', nights: 3, facts: 2 };
  const t = setup({ slime_import_preview: preview, slime_import_apply: null });
  await t.file.startImport();
  assert.ok(t.file.pending);
  assert.ok(t.shown[0].field.confirm);
  t.file.no();
  assert.equal(t.file.pending, false);
  assert.equal(t.calls.some(([c]) => c === 'slime_import_apply'), false);
  await t.file.yes();
  assert.equal(t.calls.some(([c]) => c === 'slime_import_apply'), false);
});

test('Yes applies the previewed file only', async () => {
  const preview = { file: 'a.slime', slime_name: 'Mochi', nights: 3, facts: 2 };
  const t = setup({ slime_import_preview: preview, slime_import_apply: null });
  await t.file.startImport();
  await t.file.yes();
  assert.deepEqual(t.calls.at(-1), ['slime_import_apply', { file: 'a.slime' }]);
  assert.equal(t.applied(), 1);
});

test('a refused file shows the reason and asks nothing', async () => {
  const t = setup({ slime_import_preview: new Error('slime_out_of_range') });
  await t.file.startImport();
  assert.equal(t.file.pending, false);
  assert.equal(t.shown.length, 1);
  assert.equal(t.shown[0].field, undefined);
});

test('a hostile preview is refused before any question', async () => {
  const t = setup({ slime_import_preview: { file: '../../etc/passwd.slime', slime_name: 'x', nights: 1, facts: 1 } });
  await t.file.startImport();
  assert.equal(t.file.pending, false);
});
