// The genome on the page: the look Rust computed is applied to the slime model, the day is counted
// and reported to the soul file, and the night that just passed becomes one sentence. Rust owns every
// rule and bound; this file only moves numbers.

import { normalizeModel } from './model.js';
import { say } from './i18n.js';

const FLUSH_EVERY = 60;
const NIGHT_FROM = 22;
const NIGHT_UNTIL = 5;
const ACTIVE_WINDOW = 30;
const THROW_SPEED = 400;
export const JOURNAL_SHOWN_FOR = 7000;

/** Local day number: days since 1970-01-01 in the user's time zone. */
export function localDay(now = new Date()) {
  return Math.floor((now.getTime() - now.getTimezoneOffset() * 60000) / 86400000);
}

export const isNightHour = (hour) => hour >= NIGHT_FROM || hour < NIGHT_UNTIL;

/** The base model with a look from the genome applied. The model's own validation still has the last word. */
export function applyLook(base, look) {
  if (!look) return normalizeModel(base);
  const merged = structuredClone(base);
  merged.body = { ...merged.body, radius: look.radius, shape: { ...merged.body?.shape, baseWidth: look.base_width }, breathing: { ...merged.body?.breathing, speed: look.breathing_speed } };
  merged.material = {
    ...merged.material,
    colors: look.colors,
    layers: look.layers ?? merged.material?.layers,
    opacity: look.opacity,
    edgeOpacity: look.edge_opacity,
    glow: look.glow,
    coreStrength: look.core_strength,
  };
  merged.bubbles = { ...merged.bubbles, count: look.bubbles };
  merged.face = {
    ...merged.face,
    blush: look.blush,
    eyes: { ...merged.face?.eyes, width: look.eye_width, separation: look.eye_separation, aspect: look.eye_aspect },
  };
  return normalizeModel(merged);
}

/** One sentence about the night that passed, or null when there was none. */
export function journalLine(language, night) {
  if (!night) return null;
  if (night.mutation) return say(language, 'journalMutation', { label: say(language, `gene_${night.mutation}`) });
  if (night.headline) return say(language, 'journalChange', { label: say(language, `gene_${night.headline}`) });
  return say(language, 'journalQuiet');
}

const EMPTY = () => ({ caress_seconds: 0, throws: 0, answered: 0, ignored: 0, messages: 0, active_seconds: 0, night_seconds: 0, calmed_alone: 0 });

/** Counts the day and reports it to the soul file once a minute, only when something happened. */
export function createDayCounter({ invoke, today = localDay, hour = () => new Date().getHours(), onNight = () => {} }) {
  const pending = new Map();
  let nextFlush = null, inFlight = null, observedDay = today();
  const bucket = (date = today()) => {
    if (!pending.has(date)) pending.set(date, EMPTY());
    return pending.get(date);
  };

  function flush() {
    if (inFlight) return inFlight.then((ok) => ok ? flush() : false);
    if (!pending.size) return Promise.resolve(true);
    const batch = [...pending].sort(([a], [b]) => a - b);
    pending.clear();
    inFlight = (async () => {
      for (let i = 0; i < batch.length; i++) {
        const [date, delta] = batch[i];
        let night;
        try {
          night = await invoke('genome_day', { today: date, delta });
        } catch {
          for (const [date, delta] of batch.slice(i)) {
            const retry = bucket(date);
            for (const key of Object.keys(delta)) retry[key] += delta[key];
          }
          return false;
        }
        if (night) onNight(night);
      }
      return true;
    })().finally(() => { inFlight = null; });
    return inFlight;
  }

  return {
    caress(seconds) { bucket().caress_seconds += seconds; },
    thrown() { bucket().throws += 1; },
    answered() { bucket().answered += 1; },
    ignored() { bucket().ignored += 1; },
    message() { bucket().messages += 1; },
    calmedAlone() { bucket().calmed_alone += 1; },
    tick(now, dt, idleFor) {
      const date = today();
      if (date > observedDay) {
        observedDay = date;
        bucket(date);
        nextFlush = now;
      }
      if (idleFor < ACTIVE_WINDOW && dt > 0) {
        const counts = bucket(date);
        counts.active_seconds += dt;
        if (isNightHour(hour())) counts.night_seconds += dt;
      }
      nextFlush ??= now + FLUSH_EVERY;
      if (now >= nextFlush) { nextFlush = now + FLUSH_EVERY; flush(); }
    },
    flush,
    get pending() {
      const total = EMPTY();
      for (const counts of pending.values()) for (const key of Object.keys(total)) total[key] += counts[key];
      return total;
    },
  };
}

export function createNightJournal({ dialogue, language }) {
  const waiting = [];
  let until = null;
  return {
    enqueue(night) { const line = journalLine(language, night); if (line) waiting.push(line); },
    get active() { return until !== null; },
    tick(now, busy) {
      if (until !== null && (busy || !dialogue.visible)) until = null;
      if (until !== null && now >= until) { dialogue.hide(); until = null; }
      if (until === null && !busy && !dialogue.visible && waiting.length) {
        dialogue.show({ message: waiting.shift() });
        until = now + JOURNAL_SHOWN_FOR / 1000;
      }
    },
  };
}

/** True on the frame a held slime is released fast enough to count as a throw. */
export function createThrowDetector() {
  let held = false;
  return (grabbed, speed) => {
    const thrown = held && !grabbed && speed > THROW_SPEED;
    held = grabbed;
    return thrown;
  };
}
