import test from 'node:test';
import assert from 'node:assert/strict';
import { createChat } from '../chat.js';

function setup(invoke) {
  const shown = [];
  const faces = [];
  const dialogue = { show: (v) => shown.push(v), hide: () => shown.push('hide') };
  const chat = createChat({ dialogue, invoke, language: 'fr', onExpression: (n, s) => faces.push([n, s]), isSulking: () => false });
  return { chat, shown, faces };
}

test('the reply is shown and its expression is played', async () => {
  const { chat, shown, faces } = setup(async () => ({ say: 'Coucou !', expression: 'happy', valence: 0.5, arousal: 0.3 }));
  chat.start();
  assert.equal(await chat.answer('salut'), null);
  assert.equal(shown.at(-1).message, 'Coucou !');
  assert.deepEqual(faces, [['happy', 4.5]]);
});

test('the history is sent with every turn and kept short', async () => {
  const seen = [];
  const { chat } = setup(async (_cmd, args) => { seen.push(args.history.map((t) => t.role + ':' + t.text)); return { say: 'ok', expression: null }; });
  chat.start();
  for (let i = 0; i < 10; i++) await chat.answer(`m${i}`);
  assert.ok(seen.at(-1).length <= 12);
  assert.equal(seen.at(-1).at(-1), 'user:m9');
});

test('a missing key is explained in the slime voice, not as a code', async () => {
  const { chat, shown } = setup(async () => { throw 'no_key'; });
  chat.start();
  await chat.answer('hey');
  assert.match(shown.at(-1).message, /Cerveau/);
});

test('a provider error is relayed', async () => {
  const { chat, shown } = setup(async () => { throw 'provider:model not found'; });
  chat.start();
  await chat.answer('hey');
  assert.match(shown.at(-1).message, /model not found/);
});

test('an empty message is refused and nothing is sent', async () => {
  let calls = 0;
  const { chat } = setup(async () => { calls += 1; return { say: 'x' }; });
  chat.start();
  assert.match(await chat.answer('   '), /Dis-moi/);
  assert.equal(calls, 0);
});

test('closing hides the bubble and ignores late answers', async () => {
  const { chat, shown } = setup(async () => ({ say: 'x' }));
  chat.start();
  chat.close();
  assert.equal(shown.at(-1), 'hide');
  assert.equal(await chat.answer('hello'), null);
});

test('each message sent to the brain is counted once for the day, an empty one is not', async () => {
  let counted = 0;
  const dialogue = { show() {}, hide() {} };
  const chat = createChat({ dialogue, invoke: async () => ({ say: 'ok' }), language: 'fr', onExpression() {}, isSulking: () => false, onMessage: () => { counted += 1; } });
  chat.start();
  await chat.answer('   ');
  await chat.answer('salut');
  await chat.answer('encore');
  assert.equal(counted, 2);
});

const ask = { say: 'Je regarde le fichier.', expression: 'calm', tool: { server: 'files', name: 'read', arguments: { path: 'a.txt' } } };

test('a tool request waits for a yes before anything runs, then the result goes back to the brain', async () => {
  const calls = [];
  const { chat, shown } = setup(async (cmd, args) => {
    calls.push(cmd);
    if (cmd === 'brain_chat') return calls.filter((c) => c === 'brain_chat').length === 1 ? ask : { say: 'Il dit bonjour.', expression: null };
    return { text: 'contenu', failed: false };
  });
  chat.start();
  await chat.answer('lis a.txt');
  assert.equal(chat.asking, true);
  assert.equal(shown.at(-1).field.confirm, true);
  assert.deepEqual(calls, ['brain_chat'], 'no tool ran yet');
  await chat.answer('');
  assert.deepEqual(calls, ['brain_chat', 'mcp_call', 'brain_chat']);
  assert.equal(chat.asking, false);
  assert.equal(shown.at(-1).message, 'Il dit bonjour.');
});

test('declining runs nothing and tells the brain', async () => {
  const calls = [], turns = [];
  const { chat } = setup(async (cmd, args) => {
    calls.push(cmd);
    if (cmd === 'brain_chat') { turns.push(args.history.at(-1).text); return calls.length === 1 ? ask : { say: "D'accord.", expression: null }; }
    throw new Error('must not run');
  });
  chat.start();
  await chat.answer('lis a.txt');
  await chat.decline();
  assert.deepEqual(calls, ['brain_chat', 'brain_chat']);
  assert.match(turns.at(-1), /refusé/);
  assert.equal(chat.asking, false);
});

test('a failing tool is reported to the brain as data, and closing forgets the request', async () => {
  const turns = [];
  const { chat, shown } = setup(async (cmd, args) => {
    if (cmd === 'mcp_call') throw 'no_permission';
    turns.push(args.history.at(-1).text);
    return turns.length === 1 ? ask : { say: 'Je ne peux pas.', expression: null };
  });
  chat.start();
  await chat.answer('lis');
  await chat.approve();
  assert.match(turns.at(-1), /a échoué/);
  assert.equal(shown.at(-1).message, 'Je ne peux pas.');
  chat.start();
  turns.length = 0;
  await chat.answer('encore');
  chat.close();
  assert.equal(chat.asking, false);
});

test('the clipboard is read by the slime itself, never by an MCP server', async () => {
  const calls = [];
  const clip = { say: 'Je regarde.', expression: null, tool: { server: 'slimy', name: 'clipboard', arguments: {} } };
  const { chat, shown } = setup(async (cmd) => {
    calls.push(cmd);
    if (cmd === 'clipboard_read') return 'du texte copié';
    return calls.filter((c) => c === 'brain_chat').length === 1 ? clip : { say: 'Joli texte.', expression: null };
  });
  chat.start();
  await chat.answer('que vois-tu ?');
  await chat.approve();
  assert.deepEqual(calls, ['brain_chat', 'clipboard_read', 'brain_chat']);
  assert.equal(shown.at(-1).message, 'Joli texte.');
});

test('the shared folder is read by the backend with the path the brain gave', async () => {
  const seen = [];
  const files = { say: 'Je lis.', expression: null, tool: { server: 'slimy', name: 'files', arguments: { path: 'notes/a.txt' } } };
  const { chat } = setup(async (cmd, args) => {
    seen.push([cmd, args?.path]);
    if (cmd === 'files_read') return 'contenu';
    return seen.filter((c) => c[0] === 'brain_chat').length === 1 ? files : { say: 'Ok.', expression: null };
  });
  chat.start();
  await chat.answer('lis mes notes');
  await chat.approve();
  assert.deepEqual(seen.filter((c) => c[0] === 'files_read'), [['files_read', 'notes/a.txt']]);
});

test('the pointer moves by whole pixels the brain asked for, after the yes', async () => {
  const moves = [];
  const ask = { say: 'Je te pousse un peu.', expression: null, tool: { server: 'slimy', name: 'pointer', arguments: { dx: 40.6, dy: 'oops' } } };
  const { chat } = setup(async (cmd, args) => {
    if (cmd === 'pointer_nudge') { moves.push(args); return null; }
    return moves.length ? { say: 'Voilà.', expression: null } : ask;
  });
  chat.start();
  await chat.answer('bouge');
  assert.deepEqual(moves, [], 'nothing moves before the yes');
  await chat.approve();
  assert.deepEqual(moves, [{ dx: 41, dy: 0 }]);
});

test('a screenshot rides with the next turn only, and older pictures are dropped from the history', async () => {
  const sent = [];
  const shot = { say: 'Je regarde ton écran.', expression: null, tool: { server: 'slimy', name: 'screen', arguments: {} } };
  let call = 0;
  const { chat } = setup(async (cmd, args) => {
    if (cmd === 'screen_capture') return 'QUJD';
    sent.push(args.history.map((t) => ({ text: t.text, image: t.image })));
    return ++call === 1 ? shot : { say: 'Un terminal.', expression: null };
  });
  chat.start();
  await chat.answer('que vois-tu ?');
  await chat.approve();
  const last = sent.at(-1);
  assert.equal(last.filter((t) => t.image).length, 1, 'one picture');
  assert.equal(last.at(-1).image, 'QUJD');
  assert.match(last.at(-1).text, /capture/);
});

test('asking for the screen says that the picture leaves for the AI provider', async () => {
  const { chat, shown } = setup(async () => ({ say: 'Je regarde.', expression: null, tool: { server: 'slimy', name: 'screen', arguments: {} } }));
  chat.start();
  await chat.answer('vois-tu ?');
  assert.match(shown.at(-1).message, /fournisseur/);
});
