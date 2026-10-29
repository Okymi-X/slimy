// Glue for the birth ritual: shapes the egg, reacts to touches, hatches with a burst and asks the
// two questions through the speech bubble. The story itself lives in birth.js.

import { createBirth } from './birth.js';
import { say } from './i18n.js';

const WOBBLE_EVERY = [2, 5];
const POKE_KICK = { x: 220, y: -170 };
const HATCH_LEAP = -720;
const EGG_SHAPE = { top: 1.2, bottom: 0.9, baseWidth: 0.82 };

export function createRitual({ slime, renderer, dialogue, invoke, language, model, startAt, genomeSeed = null }) {
  const birth = createBirth(startAt);
  let shownPrompt = null;
  let nextWobble = startAt + 2;
  let clock = startAt;

  slime.set_shape(EGG_SHAPE.top, EGG_SHAPE.bottom, EGG_SHAPE.baseWidth);
  renderer.setPalette('egg');

  function hatch() {
    const { shape } = model.body;
    slime.set_shape(shape.top, shape.bottom, shape.baseWidth);
    renderer.setColors(model.material.colors);
    slime.impulse(0, HATCH_LEAP);
    renderer.burst(slime.centroid_x(), slime.centroid_y());
  }

  const show = (view) => {
    if (view.prompt === shownPrompt) return;
    shownPrompt = view.prompt;
    if (!view.prompt) { dialogue.hide(); return; }
    const field = view.input
      ? { placeholder: say(language, view.prompt === 'askUser' ? 'placeholderUser' : 'placeholderSlime'), submit: say(language, 'submit') }
      : null;
    dialogue.show({ message: say(language, view.prompt, view.vars), field });
  };

  return {
    get finished() { return birth.phase === 'done'; },
    get inEgg() { return birth.phase === 'egg'; },
    poke(t) {
      if (!this.inEgg) return;
      slime.impulse(POKE_KICK.x * (Math.random() < 0.5 ? -1 : 1), POKE_KICK.y);
      if (birth.poke(t)) hatch();
    },
    /** The answer typed in the bubble. Returns an error sentence to show, or null when accepted. */
    async answer(text) {
      const result = birth.submit(text, clock);
      if (!result.ok) return say(language, result.error);
      if (!result.finished) return null;
      const { userName, slimeName } = birth.names;
      try {
        await invoke('soul_create', { userName, slimeName, language, genomeSeed });
      } catch (code) {
        return say(language, String(code));
      }
      birth.welcome(clock);
      return null;
    },
    /** Advances the ritual and returns what the renderer needs. */
    update(t) {
      clock = t;
      if (birth.update(t)) hatch();
      if (this.inEgg && t >= nextWobble) {
        slime.impulse((Math.random() - 0.5) * 140, -50);
        nextWobble = t + WOBBLE_EVERY[0] + Math.random() * (WOBBLE_EVERY[1] - WOBBLE_EVERY[0]);
      }
      const view = birth.view(t);
      show(view);
      return {
        egg: view.phase === 'egg' || (view.phase === 'hatching' && view.awake <= 0) ? { cracks: view.cracks } : null,
        awake: view.awake,
        asking: view.input,
      };
    },
  };
}
