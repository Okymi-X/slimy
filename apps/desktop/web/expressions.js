// Expressions: a small table of face poses and a state machine that picks one from what happens
// to the slime. Numbers blend smoothly so a change of mood is never a snap; the discrete parts
// (eye and mouth kinds) change through a quick pop handled by the face.

export const EXPRESSIONS = {
  calm: { eye: 'oval', mouth: 'smile', open: 1, wide: 1, tilt: 0, curve: 0.5, mw: 1, blush: 0 },
  happy: { eye: 'oval', mouth: 'smile', open: 1.05, wide: 1.05, tilt: 0, curve: 1, mw: 1.5, blush: 0.6 },
  love: { eye: 'heart', mouth: 'smile', open: 1, wide: 1.1, tilt: 0, curve: 1, mw: 1.5, blush: 1, hearts: true },
  surprised: { eye: 'oval', mouth: 'o', open: 1.15, wide: 1.18, tilt: 0, curve: 0, mw: 1, blush: 0 },
  joy: { eye: 'arch', mouth: 'open', open: 1, wide: 1, tilt: 0, curve: 1, mw: 1.7, blush: 0.8 },
  ouch: { eye: 'arch', mouth: 'wavy', open: 1, wide: 1, tilt: 0, curve: 0, mw: 1.2, blush: 0.2 },
  dizzy: { eye: 'spiral', mouth: 'wavy', open: 1, wide: 1.1, tilt: 0, curve: 0, mw: 1.2, blush: 0.2 },
  sleepy: { eye: 'line', mouth: 'smile', open: 0.4, wide: 1, tilt: 0, curve: 0.25, mw: 0.6, blush: 0, zzz: true },
  yawn: { eye: 'arch', mouth: 'yawn', open: 1, wide: 1, tilt: 0, curve: 0, mw: 1, blush: 0 },
  sulk: { eye: 'oval', mouth: 'smile', open: 0.55, wide: 0.9, tilt: 0.3, curve: -0.75, mw: 0.75, blush: 0 },
  curious: { eye: 'oval', mouth: 'o', open: 1.12, wide: 1.08, tilt: -0.1, curve: 0.2, mw: 0.7, blush: 0 },
  focus: { eye: 'oval', mouth: 'smile', open: 0.92, wide: 0.98, tilt: 0.12, curve: 0.35, mw: 0.9, blush: 0 },
  brace: { eye: 'oval', mouth: 'smile', open: 0.8, wide: 0.95, tilt: 0.2, curve: 0.1, mw: 0.8, blush: 0 },
  leap: { eye: 'oval', mouth: 'smile', open: 1.04, wide: 1.04, tilt: 0, curve: 0.45, mw: 0.9, blush: 0 },
  fond: { eye: 'oval', mouth: 'smile', open: 1.05, wide: 1.05, tilt: -0.05, curve: 0.9, mw: 1.2, blush: 0.5 },
  silly: { eye: 'oval', mouth: 'tongue', open: 1, wide: 1, tilt: 0, curve: 1, mw: 1.3, blush: 0.3 },
};

// The face of each step of a catch: it notices, gets ready, springs, braces on landing, spreads happily, holds tenderly, backs off calm.
const CATCH_FACE = {
  acquire: () => 'curious',
  prepare: (crouching) => (crouching ? 'brace' : 'focus'),
  flight: () => 'leap',
  contact: () => 'brace',
  spread: () => 'happy',
  hold: () => 'fond',
  retreat: () => 'calm',
};

const NUMBERS = ['open', 'wide', 'tilt', 'curve', 'mw', 'blush'];
const LOVE_AFTER = 1.8;
const STROKE_MIN = 40;
const LOVE_HOLD = 1.4;
const DIZZY_ACC = 6000;
const DIZZY_FOR = 1.8;
const OUCH_FOR = 0.55;
const FAST = 700;
const YAWN_FOR = 1.8;
const MIN_HOLD = 0.4;
const URGENT = new Set(['surprised', 'joy', 'ouch', 'dizzy', 'brace', 'leap']);

// Nothing here is random: every face has a cause (what the slime is doing, what the user does, what happened).
export function createExpressions() {
  const current = { ...EXPRESSIONS.calm };
  let name = 'calm';
  let petTime = 0, loveUntil = -1, dizzyUntil = -1, ouchUntil = -1, yawnUntil = -1;
  let shaken = 0, wasSleepy = false;
  let since = 0;

  function choose(input, t) {
    const { grabbed, hover, speed, airborne, sleepiness, sulking, hint, crouching } = input;
    const mood = input.intent ?? 'idle';
    if (grabbed) return speed > 300 ? 'joy' : 'surprised';
    if (t < dizzyUntil) return 'dizzy';
    if (airborne && speed > FAST) return 'joy';
    if (t < ouchUntil) return 'ouch';
    if (hint && t < hint.until && EXPRESSIONS[hint.name]) return hint.name;
    if (mood === 'peek') return 'curious';
    if (sulking) return 'sulk';
    if (hover && petTime > LOVE_AFTER) return 'love';
    if (t < loveUntil) return 'love';
    if (hover) return 'happy';
    if (t < yawnUntil) return 'yawn';
    if ((sleepiness > 0.55 && mood === 'idle') || mood === 'nap') return 'sleepy';
    if (mood === 'catch') return CATCH_FACE[input.catchPhase]?.(crouching) ?? 'focus';
    if (crouching) return 'brace';
    if (airborne && mood !== 'idle') return 'leap';
    if (mood === 'follow') return 'fond';
    if (mood === 'wander') return 'focus';
    return 'calm';
  }

  return {
    /** input: { grabbed, hover, pointerSpeed, speed, airborne, squash, sleepiness, acc, sulking }. Returns { name, pose }. */
    update(input, t, dt) {
      // Only a moving pointer strokes the slime: a cursor resting on it, or a slime landing under a still cursor, is not a caress.
      const stroking = input.hover && !input.grabbed && (input.pointerSpeed ?? 0) > STROKE_MIN;
      petTime = stroking ? petTime + dt : input.hover ? Math.max(0, petTime - dt * 0.5) : 0;
      if (petTime > LOVE_AFTER) loveUntil = t + LOVE_HOLD;
      shaken = Math.max(0, shaken * Math.exp(-dt * 3) + (Math.abs(input.acc) > DIZZY_ACC && (input.grabbed || (input.intent ?? 'idle') === 'idle') ? dt * 6 : 0));
      if (shaken > 1) { dizzyUntil = t + DIZZY_FOR; shaken = 0; }
      if ((input.impact ?? 0) > 420 && !input.grabbed && !input.crouching && t >= ouchUntil) ouchUntil = t + OUCH_FOR;
      const sleepy = (input.sleepiness > 0.55 && (input.intent ?? 'idle') === 'idle') || input.intent === 'nap';
      if (sleepy && !wasSleepy) yawnUntil = t + YAWN_FOR;
      wasSleepy = sleepy;
      const wanted = choose(input, t);
      // A calm change of mood waits a moment so a pointer on the edge of the slime cannot make the face flicker.
      if (wanted !== name && (URGENT.has(wanted) || t - since >= MIN_HOLD)) { name = wanted; since = t; }
      const target = EXPRESSIONS[name];
      const ease = 1 - Math.exp(-9 * dt);
      for (const k of NUMBERS) current[k] += (target[k] - current[k]) * ease;
      return { name, pose: current, eye: target.eye, mouth: target.mouth, hearts: !!target.hearts, zzz: !!target.zzz };
    },
  };
}
