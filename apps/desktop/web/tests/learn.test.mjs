import test from 'node:test';
import assert from 'node:assert/strict';
import { createLearning, createWatch, remarkReward, speechReward, walkReward, greetReward } from '../learn.js';
import { createWalkUrge } from '../roam.js';
import { createVoice } from '../voice.js';
import { createRemarks } from '../remarks.js';

test('a reaction inside the window is engagement, silence after it is not', () => {
  const watch = createWatch(90);
  watch.start(10, { arm: 1 });
  assert.equal(watch.tick(50), null, 'still waiting');
  assert.equal(watch.interact(60).outcome, 'engaged');
  watch.start(100, { arm: 2 });
  assert.equal(watch.tick(195).outcome, 'ignored');
  watch.start(300, { arm: 0 });
  assert.equal(watch.interact(400), null, 'too late to count');
  assert.equal(watch.active, true);
  assert.equal(watch.tick(400).outcome, 'ignored');
  watch.start(500, { arm: 1 });
  assert.equal(watch.cut().outcome, 'cut');
  assert.equal(watch.cut(), null, 'nothing left to settle');
});

test('rewards rank the outcomes sensibly and stay within -1 and 1', () => {
  assert.ok(remarkReward('engaged', 600) > remarkReward('engaged', 2400), 'a welcome remark at a lively pace is worth a little more');
  assert.ok(remarkReward('engaged', 900) > 0 && remarkReward('ignored', 900) < 0);
  assert.ok(speechReward('engaged') > speechReward('hurried') && speechReward('hurried') > speechReward('ignored') && speechReward('ignored') > speechReward('cut'));
  for (const o of ['engaged', 'hurried', 'ignored', 'cut', 'nonsense']) assert.ok(Math.abs(speechReward(o)) <= 1, o);
  for (const g of [0, 600, 99999, -5]) assert.ok(Math.abs(remarkReward('engaged', g)) <= 1 && Math.abs(remarkReward('ignored', g)) <= 1);
});

test('a missing learning model never breaks anything: the defaults are used', async () => {
  const learning = createLearning({ invoke: async () => { throw 'learn_unknown'; } });
  assert.equal(await learning.choose('remark_gap'), null);
  await assert.doesNotReject(learning.reward(null, 1));
  await assert.doesNotReject(learning.reward({ decision: 'x', hour: 1, arm: 0 }, 1));
});

const installed = { installed: { speech: true, hearing: true }, config: { replies: true, style: 'auto' } };

test('the pace of speech is learned: asked before, rewarded after the reaction', async () => {
  const log = [];
  let time = 1000;
  const invoke = async (c, a) => {
    log.push([c, a]);
    if (c === 'voice_status') return installed;
    if (c === 'learn_choose') return { arm: 2, value: 1.08 };
    return null;
  };
  const voice = createVoice({ invoke, language: 'fr', learn: createLearning({ invoke, hour: () => 14 }), clock: () => time });
  await voice.refresh();
  await voice.say('Salut !');
  assert.equal(log.find((l) => l[0] === 'voice_speak')[1].pace, 1.08, 'the learned pace is used');
  time += 8;
  await voice.engaged();
  const reward = log.find((l) => l[0] === 'learn_reward')[1];
  assert.deepEqual([reward.decision, reward.arm, reward.hour, reward.reward], ['voice_pace', 2, 14, 0.6]);
});

test('cutting the slime off is a strong no, silence a mild one, answering over it a small yes', async () => {
  const rewards = [];
  let time = 0;
  let speakingForever;
  const invoke = async (c) => {
    if (c === 'voice_status') return installed;
    if (c === 'learn_choose') return { arm: 0, value: 0.92 };
    if (c === 'learn_reward') { rewards.push(rewardOf(arguments)); return null; }
    if (c === 'voice_speak') return new Promise((r) => { speakingForever = r; });
    return null;
  };
  function rewardOf() { return null; }
  const calls = [];
  const spy = async (c, a) => { if (c === 'learn_reward') calls.push(a.reward); return invoke(c, a); };
  const voice = createVoice({ invoke: spy, language: 'fr', learn: createLearning({ invoke: spy }), clock: () => time });
  await voice.refresh();
  const saying = voice.say('Une longue histoire');
  await new Promise((r) => setTimeout(r, 5));
  voice.cut();
  await new Promise((r) => setTimeout(r, 5));
  speakingForever();
  await saying;
  assert.deepEqual(calls, [-1], 'cut while speaking');
  // answered over it
  const second = voice.say('Encore');
  await new Promise((r) => setTimeout(r, 5));
  voice.engaged();
  await new Promise((r) => setTimeout(r, 5));
  speakingForever();
  await second;
  assert.deepEqual(calls, [-1, 0.2]);
  // said, then nothing for a while
  const third = voice.say('Et puis');
  await new Promise((r) => setTimeout(r, 5));
  speakingForever();
  await third;
  time += 40;
  await voice.tick();
  assert.deepEqual(calls, [-1, 0.2, -0.1]);
});

test('the spacing of spontaneous remarks follows what the model chose, and a welcome remark is rewarded', async () => {
  const log = [];
  const invoke = async (c, a) => {
    log.push([c, a]);
    if (c === 'learn_choose') return { arm: 3, value: 2400 };
    if (c === 'brain_chat') return { say: 'Coucou.', expression: null };
    return null;
  };
  const dialogue = { shown: [], visible: false, text: '', show({ message }) { this.shown.push(message); this.visible = true; this.text = message; }, hide() { this.visible = false; } };
  const remarks = createRemarks({ dialogue, invoke, language: 'fr', hour: () => 10, learn: createLearning({ invoke, hour: () => 10 }) });
  const free = { returned: true, caressed: false, free: true, sulking: false };
  assert.equal(await remarks.update(10, free), true);
  await remarks.interaction(40);
  assert.equal(await remarks.update(1000, free), false, 'a gap of 2400 s has not passed, even though the default 900 s has');
  const rewarded = log.find((l) => l[0] === 'learn_reward');
  assert.ok(rewarded && rewarded[1].decision === 'remark_gap' && rewarded[1].reward >= 0.85);
  assert.equal(await remarks.update(2500, free), true, 'after the chosen gap it speaks again');
});
