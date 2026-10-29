import test from 'node:test';
import assert from 'node:assert/strict';
import { percent, canSpeak, canHear, createVoice } from '../voice.js';
import { createChat } from '../chat.js';
import { say, explain } from '../i18n.js';

const installed = { installed: { speech: true, hearing: true }, config: { replies: true, style: 'auto' }, megabytes: { speech: 126, hearing: 198 }, voice: { name: 'ff_siwis' } };

test('progress is a whole percent and never beyond 100', () => {
  assert.equal(percent(50, 200), 25);
  assert.equal(percent(300, 200), 100);
  assert.equal(percent(5, 0), 0);
});

test('speaking needs the voice and the wish; hearing needs the ears', () => {
  assert.equal(canSpeak(installed), true);
  assert.equal(canSpeak({ ...installed, config: { replies: false } }), false);
  assert.equal(canSpeak({ installed: { speech: false } }), false);
  assert.equal(canSpeak(null), false);
  assert.equal(canHear(installed), true);
  assert.equal(canHear({ installed: { hearing: false } }), false);
});

test('saying does nothing without a voice and never throws', async () => {
  const calls = [];
  const voice = createVoice({ invoke: async (c, a) => { calls.push([c, a]); if (c === 'voice_status') return { installed: { speech: false } }; throw 'boom'; }, language: 'fr' });
  await voice.refresh();
  await voice.say('Salut');
  assert.deepEqual(calls.map((c) => c[0]), ['voice_status']);
  const talking = createVoice({ invoke: async (c) => { if (c === 'voice_status') return installed; throw 'voice_failed'; }, language: 'fr' });
  await talking.refresh();
  await assert.doesNotReject(talking.say('Salut'));
  assert.equal(talking.speaking, false);
});

function setup(invoke, status = installed) {
  const shown = [];
  const dialogue = { show: (v) => shown.push(v), hide: () => shown.push('hide') };
  const voice = createVoice({ invoke, language: 'fr' });
  voice.status; // refreshed below
  const chat = createChat({ dialogue, invoke, language: 'fr', onExpression: () => {}, isSulking: () => false, voice });
  return { chat, shown, voice, ready: voice.refresh() };
}

test('the microphone button appears only when the ears are installed', async () => {
  const withEars = setup(async (c) => (c === 'voice_status' ? installed : null));
  await withEars.ready;
  withEars.chat.start();
  assert.equal(typeof withEars.shown.at(-1).field.mic, 'function');
  const without = setup(async (c) => (c === 'voice_status' ? { installed: { hearing: false } } : null));
  await without.ready;
  without.chat.start();
  assert.equal(without.shown.at(-1).field.mic, undefined);
});

test('what is heard is sent as a message and the answer is spoken', async () => {
  const calls = [];
  const invoke = async (c, a) => {
    calls.push(c);
    if (c === 'voice_status') return installed;
    if (c === 'voice_listen') return 'Quelle heure est-il';
    if (c === 'brain_chat') { calls.push(`heard:${a.history.at(-1).text}`); return { say: 'Il est tard.', expression: null }; }
    return null;
  };
  const { chat, shown, ready } = setup(invoke);
  await ready;
  chat.start();
  await chat.toggleMic();
  assert.ok(shown.some((s) => s.message === say('fr', 'listening') && s.field.listening), 'shows it is listening');
  assert.ok(calls.includes('heard:Quelle heure est-il'));
  assert.equal(shown.at(-1).message, 'Il est tard.');
  await new Promise((r) => setTimeout(r, 5));
  assert.ok(calls.includes('voice_speak'), 'the reply is said aloud');
});

test('listening problems are explained, and a missing power says where to grant it', async () => {
  for (const [error, expected] of [['no_permission', say('fr', 'micNeedsPower')], ['voice_nobody', say('fr', 'voice_nobody')], ['voice_no_mic', say('fr', 'voice_no_mic')]]) {
    const { chat, shown, ready } = setup(async (c) => { if (c === 'voice_status') return installed; throw error; });
    await ready;
    chat.start();
    await chat.toggleMic();
    assert.equal(shown.at(-1).message, expected, error);
  }
});

test('a second press stops the recording, and closing stops the voice', async () => {
  const calls = [];
  let finish;
  const invoke = async (c) => {
    calls.push(c);
    if (c === 'voice_status') return installed;
    if (c === 'voice_listen') return new Promise((r) => { finish = r; });
    return null;
  };
  const { chat, ready } = setup(invoke);
  await ready;
  chat.start();
  const first = chat.toggleMic();
  await new Promise((r) => setTimeout(r, 1));
  await chat.toggleMic();
  assert.ok(calls.includes('voice_stop_listening'));
  finish('stop');
  await first.catch(() => {});
  chat.close();
  assert.ok(calls.includes('voice_stop_speaking'));
});

test('every voice text exists in both languages', () => {
  const keys = ['voiceHearingSmall', 'voiceModel', 'voiceModel_base', 'voiceModel_small', 'voiceNotInstalled', 'micLabel', 'listening', 'micNeedsPower', 'voice_missing', 'voice_nobody', 'voice_no_mic', 'voice_no_speaker', 'voice_failed', 'voice_model', 'voice_install', 'voice_style',
    'voiceTitle', 'voiceUnavailable', 'voiceHearing', 'voiceSpeech', 'voiceInstalled', 'voiceInstall', 'voiceReplies', 'voiceStyle', 'voiceStyle_auto', 'voiceStyle_kawaii', 'voiceStyle_calm', 'voiceName', 'power_microphone', 'learnedTitle', 'learnedNothing', 'learnedForget', 'learnedGap', 'learnedPace', 'part_night', 'part_morning', 'part_afternoon', 'part_evening', 'learn_unknown', 'displayTitle', 'displayScreenLabel', 'displayPrimary', 'displayFollow', 'displayScreen'];
  for (const language of ['fr', 'en']) for (const key of keys) assert.notEqual(say(language, key), key, `${language} ${key}`);
  for (const code of ['voice_missing', 'voice_nobody', 'voice_no_mic', 'voice_no_speaker', 'voice_failed', 'voice_model', 'voice_install', 'voice_style']) assert.notEqual(explain('en', code), code);
});

test('the models are warmed up in the background only when something is installed', async () => {
  const calls = [];
  const make = (status) => createVoice({ invoke: async (c) => { calls.push(c); return c === 'voice_status' ? status : null; }, language: 'fr' });
  const none = make({ installed: { speech: false, hearing: false } });
  await none.refresh(); none.warm(1);
  const some = make(installed);
  await some.refresh(); some.warm(1);
  await new Promise((r) => setTimeout(r, 30));
  assert.equal(calls.filter((c) => c === 'voice_warm').length, 1);
});

test('the answer is spoken with the feeling the brain gave it', async () => {
  const sent = [];
  const invoke = async (c, a) => {
    if (c === 'voice_status') return installed;
    if (c === 'voice_speak') { sent.push(a); return null; }
    return null;
  };
  const voice = createVoice({ invoke, language: 'fr' });
  await voice.refresh();
  await voice.say('Super !', { expression: 'joy', valence: 0.8, arousal: 0.9 });
  await voice.say('Bonjour');
  assert.deepEqual(sent[0], { text: 'Super !', language: 'fr', expression: 'joy', valence: 0.8, arousal: 0.9, pace: 1 });
  assert.deepEqual(sent[1], { text: 'Bonjour', language: 'fr', expression: null, valence: null, arousal: null, pace: 1 });
});
