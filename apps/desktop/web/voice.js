// The slime's voice on the page: asks the backend to speak its answers and to listen through the microphone. All the
// audio and the models live in the backend; this only decides when to ask and keeps the state.

/** Download progress as a whole percent. */
export const percent = (done, total) => (total > 0 ? Math.min(100, Math.round((done / total) * 100)) : 0);

/** The slime can say its answers: the voice is installed and the user wants spoken replies. */
export const canSpeak = (status) => !!status?.installed?.speech && status?.config?.replies !== false;

/** The slime can listen: the hearing model is installed (the microphone power is checked by the backend). */
export const canHear = (status) => !!status?.installed?.hearing;

import { createWatch, speechReward, SPEECH_WINDOW } from './learn.js';

export function createVoice({ invoke, language, learn = null, clock = () => Date.now() / 1000 }) {
  let status = null;
  let speaking = false;
  let hearing = false;
  // The pace of what is said is chosen by the tiny learning model, then rewarded by how the person reacts.
  let paceChoice = null;
  const watch = createWatch(SPEECH_WINDOW);
  const settle = (done) => (done ? learn?.reward(done.choice, speechReward(done.outcome)) : undefined);
  return {
    get status() { return status; },
    get speaking() { return speaking; },
    get hearing() { return hearing; },
    async refresh() {
      status = await invoke('voice_status', { language }).catch(() => null);
      return status;
    },
    /** Loads the models in the background a little after start, so the first word comes quickly. */
    warm(delayMs = 8000) {
      if (!canSpeak(status) && !canHear(status)) return;
      setTimeout(() => { invoke('voice_warm', { language }).catch(() => {}); }, delayMs);
    },
    /** Says the text aloud, if there is a voice; never throws, a missing voice is just silence. */
    async say(text, mood = {}) {
      if (!canSpeak(status) || !String(text ?? '').trim()) return;
      await settle(watch.tick(clock()));
      paceChoice = await learn?.choose('voice_pace') ?? null;
      const mine = paceChoice;
      speaking = true;
      try { await invoke('voice_speak', { text: String(text), language, expression: mood.expression ?? null, valence: mood.valence ?? null, arousal: mood.arousal ?? null, pace: mine?.value ?? 1 }); } catch { /* silence */ } finally { speaking = false; }
      // Still ours (not cut or answered over): now see whether the person reacts.
      if (paceChoice === mine && mine) { watch.start(clock(), mine); paceChoice = null; }
    },
    /** The person wrote or spoke: a reaction to what was said (over it when the slime was still talking). */
    engaged() {
      if (speaking && paceChoice) { const choice = paceChoice; paceChoice = null; return settle({ choice, outcome: 'hurried' }); }
      return settle(watch.interact(clock()));
    },
    /** The person stopped the slime while it was talking: the clearest sign the pace or the length was wrong. */
    cut() {
      if (speaking && paceChoice) { const choice = paceChoice; paceChoice = null; return settle({ choice, outcome: 'cut' }); }
      return undefined;
    },
    /** Time passing, so an unanswered sentence is settled. */
    tick() { return settle(watch.tick(clock())); },
    stopSpeaking() { return invoke('voice_stop_speaking').catch(() => {}); },
    /** Listens until the person has spoken and paused. Resolves with the words; rejects with the backend's error code. */
    async hear() {
      hearing = true;
      try { return await invoke('voice_listen', { language }); } finally { hearing = false; }
    },
    stopHearing() { return invoke('voice_stop_listening').catch(() => {}); },
  };
}
