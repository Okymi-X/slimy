// What the slime learns from its human: two small choices (how long to wait before speaking up on its own, how fast to
// talk) are made by a tiny local model in the backend, and this file watches how the person reacts and tells it.

export const REMARK_WINDOW = 90; // seconds to react to a spontaneous remark
export const SPEECH_WINDOW = 25; // seconds to answer something the slime said aloud
export const WALK_WINDOW = 60; // seconds to react to a wander
export const GREET_WINDOW = 90; // seconds to react to the greeting on a return

/** How good a wander was: being touched or written to soon after is good, being shooed away is the clearest no, silence a mild no. */
export const walkReward = (outcome) => ({ engaged: 0.8, cut: -1, ignored: -0.2 })[outcome] ?? 0;

/** How good the greeting style was. Saying nothing and still being reacted to is only a small yes. */
export const greetReward = (outcome, style) => (outcome === 'engaged' ? (style === 0 ? 0.3 : 0.9) : style === 0 ? 0 : -0.3);

/** How good a spontaneous remark was: a reaction is good (a little more when it was not too frequent), silence is a mild no. */
export const remarkReward = (outcome, gapSeconds) => (outcome === 'engaged' ? 0.85 + 0.15 * (1 - Math.min(1, Math.max(0, gapSeconds) / 2400)) : -0.3);

/** How good the pace of a spoken answer was: cutting it off is the clearest no, answering after it the clearest yes. */
export const speechReward = (outcome) => ({ engaged: 0.6, hurried: 0.2, cut: -1, ignored: -0.1 })[outcome] ?? 0;

export function createLearning({ invoke, hour = () => new Date().getHours() }) {
  return {
    /** The option to use for a decision: { arm, value, hour }, or null when the model is not there (the caller then uses its default). */
    async choose(decision) {
      try {
        const h = hour();
        const { arm, value } = await invoke('learn_choose', { decision, hour: h });
        return Number.isFinite(value) ? { decision, arm, value, hour: h } : null;
      } catch { return null; }
    },
    async reward(choice, reward) {
      if (!choice) return;
      try { await invoke('learn_reward', { decision: choice.decision, hour: choice.hour, arm: choice.arm, reward }); } catch { /* learning is optional */ }
    },
  };
}

/** Watches one choice after it was acted on: the person reacts within the window, or does not. One choice at a time. */
export function createWatch(windowSeconds) {
  let current = null;
  const settle = (outcome) => { const done = current; current = null; return done ? { choice: done.choice, outcome, extra: done.extra } : null; };
  return {
    get active() { return current !== null; },
    start(now, choice, extra) { if (choice) current = { at: now, choice, extra }; },
    /** The person did something: settles the watch as engaged when it is still within the window. */
    interact(now) { return current && now - current.at <= windowSeconds ? settle('engaged') : null; },
    /** Time passing: settles as ignored once the window is over. */
    tick(now) { return current && now - current.at > windowSeconds ? settle('ignored') : null; },
    /** The person stopped it on purpose. */
    cut() { return current ? settle('cut') : null; },
    /** The person answered while it was still going. */
    hurry() { return current ? settle('hurried') : null; },
  };
}
