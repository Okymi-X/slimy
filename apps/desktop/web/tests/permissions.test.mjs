import test from 'node:test';
import assert from 'node:assert/strict';
import { powerRows, isWatching } from '../permissions.js';
import { say, explain } from '../i18n.js';

const state = { powers: ['active_app', 'screen', 'tool'], sensitive: ['screen'], active: ['tool'] };

test('rows tell which powers are on and which need a duration', () => {
  assert.deepEqual(powerRows(state), [
    { power: 'active_app', on: false, timed: false },
    { power: 'screen', on: false, timed: true },
    { power: 'tool', on: true, timed: false },
  ]);
  assert.deepEqual(powerRows({}), []);
});

test('the watching badge shows only while a power is granted', () => {
  assert.equal(isWatching(state), true);
  assert.equal(isWatching({ active: [] }), false);
  assert.equal(isWatching(null), false);
});

test('every power and verdict has a label in both languages', () => {
  for (const language of ['fr', 'en']) {
    for (const key of ['active_app', 'clipboard', 'screen', 'files', 'tool', 'pointer', 'microphone']) assert.notEqual(say(language, `power_${key}`), `power_${key}`);
    for (const key of ['allowed', 'denied', 'expired']) assert.notEqual(say(language, `verdict_${key}`), `verdict_${key}`);
    for (const key of ['permsTitle', 'permsHint', 'permsLog', 'permsGrant', 'permsRevoke', 'permsRevokeAll', 'permsMenu', 'watching']) assert.notEqual(say(language, key), key);
  }
});

test('tool and permission errors are explained in both languages', () => {
  for (const language of ['fr', 'en']) {
    for (const code of ['no_permission', 'mcp_config', 'mcp_unknown', 'mcp_start', 'mcp_io', 'mcp_timeout', 'mcp_reply', 'mcp_name', 'screenWarn', 'toolScreen', 'screen_failed', 'screen_denied', 'screen_timeout', 'bad_image', 'pointer_denied', 'pointer_timeout', 'pointer_unsupported', 'files_root', 'files_path', 'files_missing', 'files_binary', 'clipboard_empty', 'duration', 'power']) assert.notEqual(explain(language, code), code);
    assert.match(explain(language, 'mcp_server:boom'), /boom/);
  }
});
