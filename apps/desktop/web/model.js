// Slime model: the data that makes a slime look and move the way it does. A skin or a new
// species is another model file. Every value is validated and clamped on load, so a broken or
// hostile file can only produce an odd slime, never an error.

const COLOR_KEYS = ['top', 'mid', 'bot', 'light', 'core'];

const num = (value, fallback, min, max) => (Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback);
const flag = (value, fallback) => (typeof value === 'boolean' ? value : fallback);
const color = (value, fallback) => (Array.isArray(value) && value.length === 3 ? value.map((v, i) => num(v, fallback[i], 0, 1)) : fallback);
const css = (value, fallback) => (typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value) ? value : fallback);
const pair = (value, fallback, min, max) => (Array.isArray(value) && value.length === 2 ? value.map((v, i) => num(v, fallback[i], min, max)) : fallback);

export const BASE_COLORS = {
  top: [0.55, 0.86, 1.0], mid: [0.2, 0.58, 0.95], bot: [0.08, 0.32, 0.85], light: [0.8, 0.94, 1.0], core: [0.06, 0.25, 0.73],
};

export function normalizeModel(raw = {}) {
  const body = raw.body ?? {}, shape = body.shape ?? {}, breath = body.breathing ?? {};
  const material = raw.material ?? {}, colors = material.colors ?? {};
  const bubbles = raw.bubbles ?? {};
  const face = raw.face ?? {}, eyes = face.eyes ?? {}, mouth = face.mouth ?? {}, look = face.look ?? {}, lag = face.lag ?? {}, blink = face.blink ?? {};
  return {
    id: typeof raw.id === 'string' ? raw.id : 'unnamed',
    name: typeof raw.name === 'string' ? raw.name : 'Slime',
    body: {
      radius: num(body.radius, 64, 24, 160),
      shape: { top: num(shape.top, 0.9, 0.4, 1.4), bottom: num(shape.bottom, 0.5, 0.2, 1.0), baseWidth: num(shape.baseWidth, 1.04, 0.8, 1.4) },
      breathing: { speed: num(breath.speed, 1.7, 0, 6), width: num(breath.width, 0.016, 0, 0.08), height: num(breath.height, 0.026, 0, 0.1) },
    },
    material: {
      colors: Object.fromEntries(COLOR_KEYS.map((k) => [k, color(colors[k], BASE_COLORS[k])])),
      layers: Object.fromEntries(['stripes', 'honey', 'crystal', 'fur', 'lava', 'stars'].map((key) => [key, num(material.layers?.[key], 0, 0, 1)])),
      opacity: num(material.opacity, 0.6, 0.2, 0.95),
      edgeOpacity: num(material.edgeOpacity, 0.3, 0, 0.6),
      glow: num(material.glow, 0.3, 0, 0.8),
      gloss: num(material.gloss, 1, 0, 2),
      ripple: num(material.ripple, 0.06, 0, 0.2),
      thickness: num(material.thickness, 0.42, 0.2, 0.7),
      coreRadius: num(material.coreRadius, 0.45, 0.1, 1),
      coreStrength: num(material.coreStrength, 0.6, 0, 1),
    },
    bubbles: { count: Math.round(num(bubbles.count, 12, 0, 12)), scale: num(bubbles.scale, 1, 0.4, 2), rise: num(bubbles.rise, 1, 0.2, 3) },
    face: {
      eyes: {
        width: num(eyes.width, 0.055, 0.02, 0.12), aspect: num(eyes.aspect, 1.45, 0.6, 2.4),
        separation: num(eyes.separation, 0.17, 0.08, 0.3), height: num(eyes.height, 0.54, 0.3, 0.8), color: css(eyes.color, '#10213b'),
      },
      mouth: { width: num(mouth.width, 0.04, 0.01, 0.12), drop: num(mouth.drop, 0.13, 0.05, 0.3) },
      blush: flag(face.blush, true),
      look: { range: pair(look.range, [0.17, 0.12], 0, 0.3), follow: num(look.follow, 10, 2, 30) },
      lag: {
        velocity: num(lag.velocity, 0.011, 0, 0.05), inertia: num(lag.inertia, 0.6, 0, 2),
        stiffness: num(lag.stiffness, 120, 20, 400), damping: num(lag.damping, 13, 2, 40),
      },
      blink: { min: num(blink.min, 2.5, 0.8, 20), max: num(blink.max, 6, 1, 30) },
    },
    motion: { idleGaze: flag(raw.motion?.idleGaze, true) },
    accessories: Array.isArray(raw.accessories) ? raw.accessories.slice(0, 8) : [],
  };
}

export async function loadModel(url) {
  try {
    const response = await fetch(url);
    return normalizeModel(await response.json());
  } catch {
    return normalizeModel();
  }
}
