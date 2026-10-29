// The birth ritual: an egg that cracks when touched (or by itself after a while), a hatching, then
// the slime asks for the user's name and for its own. Pure state, the page drives the effects.

export const MAX_NAME = 24;
const CRACKS_TO_HATCH = 3;
const AUTO_CRACK_EVERY = 7;
const HATCH_TIME = 1;
const WAKE_TIME = 0.9;
const WELCOME_TIME = 4;

/** Mirrors the Rust validation, which stays authoritative. Returns an error code or null. */
export function nameError(raw) {
  const text = String(raw ?? '').trim();
  if (!text) return 'empty';
  if ([...text].some((c) => /\p{Cc}/u.test(c))) return 'bad_character';
  if ([...text].length > MAX_NAME) return 'too_long';
  return null;
}

export function createBirth(startAt = 0) {
  let phase = 'egg';
  let cracks = 0;
  let nextCrack = startAt + AUTO_CRACK_EVERY;
  let phaseSince = startAt;
  let userName = '', slimeName = '';

  function enter(next, t) { phase = next; phaseSince = t; }

  return {
    get phase() { return phase; },
    get names() { return { userName, slimeName }; },
    /** A touch on the egg. Returns true when this touch makes it hatch. */
    poke(t) {
      if (phase !== 'egg') return false;
      cracks += 1;
      nextCrack = t + AUTO_CRACK_EVERY;
      if (cracks >= CRACKS_TO_HATCH) { enter('hatching', t); return true; }
      return false;
    },
    /** Advances time. Returns true on the frame the egg hatches by itself. */
    update(t) {
      if (phase === 'egg' && t >= nextCrack) {
        nextCrack = t + AUTO_CRACK_EVERY;
        cracks += 1;
        if (cracks >= CRACKS_TO_HATCH) { enter('hatching', t); return true; }
      }
      if (phase === 'hatching' && t - phaseSince >= HATCH_TIME) enter('askUser', t);
      if (phase === 'welcome' && t - phaseSince >= WELCOME_TIME) enter('done', t);
      return false;
    },
    /** Answer to the current question. Returns { ok, error?, finished? }; the caller stores the soul when finished. */
    submit(text, t) {
      if (phase !== 'askUser' && phase !== 'askSlime') return { ok: false, error: 'not_now' };
      const error = nameError(text);
      if (error) return { ok: false, error };
      if (phase === 'askUser') {
        userName = String(text).trim();
        enter('askSlime', t);
        return { ok: true, finished: false };
      }
      slimeName = String(text).trim();
      return { ok: true, finished: true };
    },
    /** The caller confirms the soul is stored. */
    welcome(t) { if (phase === 'askSlime') enter('welcome', t); },
    view(t) {
      const asking = phase === 'askUser' || phase === 'askSlime';
      return {
        phase,
        cracks,
        awake: phase === 'egg' ? 0 : phase === 'hatching' ? Math.max(0, (t - phaseSince - 0.4) / WAKE_TIME) : 1,
        prompt: asking ? phase : phase === 'welcome' ? 'welcome' : null,
        input: asking,
        vars: { user: userName, slime: slimeName },
      };
    },
  };
}
