// Everyday life after the birth: caresses warm the slime up, silence leaves it sulking, and now and
// then it asks one small question about its human. The feelings and the choice of questions live in
// the companion (Rust); this glue talks to the user and to the soul file.

import { say } from './i18n.js';

const CHECK_EVERY = 4;
const CARESS_STEP = 1;
const RETURN_AFTER = 900;
const FRESH = 20;
const SAVE_EVERY = 60;
const THANKS_FOR = 2.6;
const ANSWER_LENGTH = 120;

export function createLife({ companion, dialogue, invoke, language, questionKey, random = Math.random, startAt, onCaress, onAnswered, onIgnored, onCalmedAlone }) {
  let asking = null;
  let stroke = 0, caressedAt = -1e9, returnedAt = -1e9;
  let lastActive = startAt, lastNow = startAt, nextCheck = startAt + CHECK_EVERY;
  let thanksUntil = -1, saved = null, nextSave = startAt + SAVE_EVERY;
  let clock = startAt;
  let wasSulking = false, sulkSince = 0;

  const feelings = () => [companion.affection(), companion.trust(), companion.resentment()];

  function show(id) {
    asking = id;
    dialogue.show({
      message: say(language, `q_${id}`),
      field: { placeholder: say(language, 'placeholderAnswer'), submit: say(language, 'submit'), later: say(language, 'later'), maxLength: ANSWER_LENGTH },
    });
  }

  return {
    get affection() { return companion.affection(); },
    /** What just happened to the slime, for moments worth a remark. */
    get moment() { return { returned: clock - returnedAt < FRESH, caressed: clock - caressedAt < FRESH }; },
    get asking() { return asking !== null || clock < thanksUntil; },
    /** The answer typed in the bubble. Returns an error sentence, or null when accepted. */
    async answer(text) {
      if (!asking) return null;
      const value = String(text ?? '').trim();
      if (!value) return say(language, 'emptyAnswer');
      try {
        await invoke('soul_learn', { key: questionKey(asking), value });
      } catch (code) {
        return say(language, String(code) === 'invalid' ? 'bad_character' : 'io');
      }
      companion.answer(asking, clock);
      onAnswered?.();
      asking = null;
      thanksUntil = clock + THANKS_FOR;
      dialogue.show({ message: say(language, `thanks_${Math.floor(random() * 4)}`) });
      return null;
    },
    /** "Later": no penalty, the question waits a couple of hours. */
    later() {
      if (!asking) return;
      companion.decline(asking, clock);
      asking = null;
      dialogue.hide();
    },
    /** input: { stroking, active, idleFor }. Returns { sulking }. */
    update(now, input) {
      clock = now;
      const dt = Math.min(now - lastNow, 0.5);
      lastNow = now;
      companion.tick(dt);

      if (input.active) {
        if (now - lastActive >= RETURN_AFTER) returnedAt = now;
        lastActive = now;
      }
      if (input.stroking) {
        stroke += dt;
        if (stroke >= CARESS_STEP) { companion.caress(stroke); onCaress?.(stroke); caressedAt = now; stroke = 0; }
      } else stroke = Math.max(0, stroke - dt);

      if (!asking && thanksUntil > 0 && now >= thanksUntil) { thanksUntil = -1; dialogue.hide(); }

      if (now >= nextCheck) {
        nextCheck = now + CHECK_EVERY;
        if (companion.check_ignored(now)) { asking = null; dialogue.hide(); onIgnored?.(); }
        if (!asking && thanksUntil < 0 && !input.busy) {
          const id = companion.ask(now, input.idleFor, now - returnedAt < FRESH, now - caressedAt < FRESH);
          if (id) show(id);
        }
      }

      if (now >= nextSave) {
        nextSave = now + SAVE_EVERY;
        const current = feelings();
        if (!saved || current.some((v, i) => Math.abs(v - saved[i]) > 0.01)) {
          saved = current;
          invoke('soul_feelings', { affection: current[0], trust: current[1], resentment: current[2] }).catch(() => {});
        }
      }
      const sulking = companion.sulking(now);
      if (sulking && !wasSulking) sulkSince = now;
      if (!sulking && wasSulking && caressedAt < sulkSince) onCalmedAlone?.();
      wasSulking = sulking;
      return { sulking };
    },
  };
}
