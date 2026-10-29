// Metamorphosis: the slime takes the silhouette of the brain it is wired to (a pixel crab for Claude, a
// cloud with a prompt for Codex and OpenAI, a whale for DeepSeek), and grows accessories from the
// materials its genome has developed (cat ears from fur, crystals, honeycomb). The silhouette is a
// radial profile, one multiplier per outline point, that the soft body glides toward.

const TAU = Math.PI * 2;
export const POINTS = 56;

// Groq (the default brain) and any custom endpoint keep the plain slime.
export const FORM_OF_PROVIDER = { anthropic: 'crab', openai: 'cloud', deepseek: 'whale', xai: 'ufo', ollama: 'llama', openrouter: 'bat', bedrock: 'rock' };
export const formForProvider = (id) => FORM_OF_PROVIDER[id] ?? 'slime';

const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));
const lobe = (a, at, width, height) => height * Math.exp(-((wrap(a - at) / width) ** 2));
const squircle = (c, s, w, h, n = 4) => 1 / ((Math.abs(c) / w) ** n + (Math.abs(s) / h) ** n) ** (1 / n);
const ellipse = (c, s, w, h) => 1 / Math.sqrt((c / w) ** 2 + (s / h) ** 2);

// Angle 0 points right and angles grow toward the floor, as in the body: the floor is at pi / 2.
const SHAPES = {
  slime: () => 1,
  // The pixel crab: a rounded block with two claws at mid height and four little legs.
  crab: (a, c, s) => squircle(c, s, 1.1, 0.95) + lobe(a, 0.12, 0.3, 0.5) + lobe(a, Math.PI - 0.12, 0.3, 0.5)
    + [0.42, 0.95, Math.PI - 0.42, Math.PI - 0.95].reduce((sum, at) => sum + lobe(a, at, 0.14, 0.34), 0),
  // The cloud: scalloped lobes, taller on top, over a flat base.
  cloud: (a, c, s) => {
    const scallops = (1 + 0.17 * Math.cos(6 * a - 0.3)) * (1 + 0.25 * Math.max(0, -s));
    return s > 0.3 ? Math.min(scallops, 0.95 / s) : scallops;
  },
  // The whale: rounded body, lighter belly, tail raised at the back.
  whale: (a, c, s) => ellipse(c, s, 1.0, 0.95) * (s > 0 ? 0.95 : 1) + lobe(a, -0.5, 0.3, 0.7) + lobe(a, Math.PI + 0.35, 0.4, 0.22),
  // The saucer hat: a rounded body under a hat.
  ufo: (a, c, s) => ellipse(c, s, 1.0, 0.95),
  // The llama: a tall, narrow body.
  llama: (a, c, s) => ellipse(c, s, 0.85, 1.1) + lobe(a, -1.2, 0.34, 0.85),
  // The bat: a round body between two wings.
  bat: (a, c, s) => ellipse(c, s, 1.0, 0.9),
  // The rock: a boxy, heavy body.
  rock: (a, c, s) => squircle(c, s, 1.0, 0.95, 3),
};

// The base dome the profile is applied to: [top, bottom, base width]. The plain slime keeps its model's own.
const BASES = { crab: [0.85, 0.85, 1], cloud: [1, 0.65, 1], whale: [0.95, 0.8, 1], ufo: [0.95, 0.7, 1], llama: [1, 0.85, 1], bat: [0.95, 0.75, 1], rock: [0.85, 0.8, 1] };
export const formShape = (name, plain) => BASES[name] ?? plain;

// Forms keep about the plain slime's volume (a bit more), so a metamorphosis changes the shape, not the size.
const AREA = 1.1;

export function profile(name, n = POINTS) {
  const shape = SHAPES[name] ?? SHAPES.slime;
  const raw = Array.from({ length: n }, (_, i) => {
    const a = (i / n) * TAU;
    return shape(a, Math.cos(a), Math.sin(a));
  });
  if (shape === SHAPES.slime) return Float32Array.from(raw);
  const fit = Math.sqrt(AREA / (raw.reduce((sum, k) => sum + k * k, 0) / n));
  return Float32Array.from(raw, (k) => k * fit);
}

// Brand colors: the crab's clay orange, the cloud's lilac to blue, the whale's blue.
const COLORS = {
  crab: { top: [0.98, 0.72, 0.6], mid: [0.87, 0.45, 0.34], bot: [0.7, 0.3, 0.22], light: [1, 0.88, 0.8], core: [0.6, 0.22, 0.15] },
  cloud: { top: [0.66, 0.64, 1], mid: [0.42, 0.52, 1], bot: [0.24, 0.3, 1], light: [0.85, 0.86, 1], core: [0.15, 0.2, 0.85] },
  whale: { top: [0.55, 0.65, 1], mid: [0.3, 0.42, 1], bot: [0.17, 0.27, 0.9], light: [0.82, 0.87, 1], core: [0.1, 0.18, 0.75] },
  ufo: { top: [0.55, 0.55, 0.62], mid: [0.2, 0.2, 0.26], bot: [0.08, 0.08, 0.12], light: [0.92, 0.92, 0.98], core: [0.02, 0.02, 0.06] },
  llama: { top: [1, 0.98, 0.94], mid: [0.93, 0.88, 0.8], bot: [0.75, 0.68, 0.58], light: [1, 1, 1], core: [0.6, 0.5, 0.4] },
  bat: { top: [0.7, 0.5, 1], mid: [0.45, 0.28, 0.85], bot: [0.25, 0.12, 0.55], light: [0.9, 0.8, 1], core: [0.15, 0.07, 0.4] },
  rock: { top: [0.7, 0.78, 0.85], mid: [0.4, 0.5, 0.62], bot: [0.2, 0.28, 0.4], light: [0.9, 0.95, 1], core: [0.12, 0.18, 0.3] },
};
/** The color of the face on a form: a dark gel needs a light face to stay readable (null keeps the model's). */
const INK = { ufo: '#eef2ff' };
export const formInk = (name) => INK[name] ?? null;

/** The colors of a form, or null to keep the slime's own genome colors. */
export const formColors = (name) => COLORS[name] ?? null;

/** How far (px) the limbs of a form stick out of the body sideways: the roamer keeps this much from the screen edges. */
const INSET = { crab: 30, whale: 56, cloud: 8, ufo: 34, llama: 14, bat: 80, rock: 14 };
export const formInset = (name) => INSET[name] ?? 0;

const SHOW_FROM = 0.4;
const rgba = (c, a = 1, k = 1) => `rgba(${Math.round(Math.min(1, c[0] * k) * 255)},${Math.round(Math.min(1, c[1] * k) * 255)},${Math.round(Math.min(1, c[2] * k) * 255)},${a})`;

function ears(ctx, m, amount, skin) {
  const size = m.w * (0.14 + 0.08 * amount);
  for (const side of [-1, 1]) {
    const x = m.cx + side * m.w * 0.27, y = m.minY + m.h * 0.12;
    ctx.beginPath();
    ctx.moveTo(x - side * size * 0.9, y + size * 0.2);
    ctx.lineTo(x + side * size * 0.1, y - size * 1.25);
    ctx.lineTo(x + side * size * 0.95, y + size * 0.35);
    ctx.closePath();
    ctx.fillStyle = rgba(skin, 0.92, 0.95);
    ctx.fill();
    ctx.beginPath();
    ctx.moveTo(x - side * size * 0.35, y + size * 0.15);
    ctx.lineTo(x + side * size * 0.12, y - size * 0.7);
    ctx.lineTo(x + side * size * 0.5, y + size * 0.2);
    ctx.closePath();
    ctx.fillStyle = 'rgba(255,214,224,.75)';
    ctx.fill();
  }
}

function crystals(ctx, m, amount) {
  const count = amount > 0.75 ? 5 : 3;
  for (let i = 0; i < count; i++) {
    const t = count === 1 ? 0.5 : i / (count - 1);
    const x = m.cx + (t - 0.5) * m.w * 0.62;
    const lift = 1 - Math.abs(t - 0.5) * 1.1;
    const base = m.minY + m.h * (0.1 + 0.18 * Math.abs(t - 0.5));
    const tall = m.h * (0.3 + 0.25 * amount) * lift, half = m.w * 0.055;
    const g = ctx.createLinearGradient(x, base - tall, x, base);
    g.addColorStop(0, 'rgba(225,250,255,.95)');
    g.addColorStop(1, 'rgba(120,200,255,.7)');
    ctx.beginPath();
    ctx.moveTo(x - half, base);
    ctx.lineTo(x - half * 0.35, base - tall * 0.8);
    ctx.lineTo(x, base - tall);
    ctx.lineTo(x + half * 0.35, base - tall * 0.8);
    ctx.lineTo(x + half, base);
    ctx.closePath();
    ctx.fillStyle = g;
    ctx.fill();
    ctx.strokeStyle = 'rgba(255,255,255,.8)';
    ctx.lineWidth = 1.2;
    ctx.stroke();
  }
}

function honeycomb(ctx, m, amount) {
  const r = m.w * (0.065 + 0.03 * amount);
  const cells = [[0, 0], [1.75, 0.5], [0.85, 1.5]];
  ctx.lineJoin = 'round';
  for (const [dx, dy] of cells) {
    const x = m.cx - m.w * 0.1 + dx * r, y = m.minY + m.h * 0.1 + dy * r - r;
    ctx.beginPath();
    for (let k = 0; k < 6; k++) {
      const a = (k / 6) * TAU + Math.PI / 6;
      ctx.lineTo(x + Math.cos(a) * r, y + Math.sin(a) * r);
    }
    ctx.closePath();
    ctx.fillStyle = 'rgba(150,84,28,.92)';
    ctx.fill();
    ctx.strokeStyle = 'rgba(255,205,120,.9)';
    ctx.lineWidth = 1.6;
    ctx.stroke();
  }
}

// A fiery slime grows two curved horns, glowing from a pale tip to an ember base. Written once for the right side and mirrored.
function horns(ctx, m, amount) {
  const tall = m.h * (0.22 + 0.2 * amount), base = m.w * (0.06 + 0.03 * amount), lean = m.w * 0.08;
  for (const s of [-1, 1]) {
    const x = m.cx + s * m.w * 0.25, y = m.minY + m.h * 0.14;
    const tipX = x + s * lean, tipY = y - tall;
    const g = ctx.createLinearGradient(x, y, tipX, tipY);
    g.addColorStop(0, 'rgba(210,80,40,.92)');
    g.addColorStop(1, 'rgba(255,225,170,.96)');
    ctx.beginPath();
    ctx.moveTo(x - s * base, y + base * 0.3);
    ctx.quadraticCurveTo(x - s * base * 0.9, y - tall * 0.55, tipX, tipY);
    ctx.quadraticCurveTo(x + s * base * 1.5 + s * lean * 0.6, y - tall * 0.35, x + s * base, y + base * 0.3);
    ctx.closePath();
    ctx.fillStyle = g;
    ctx.fill();
    ctx.strokeStyle = 'rgba(255,240,210,.8)';
    ctx.lineWidth = 1.2;
    ctx.stroke();
  }
}

// A striped slime sprouts two small leaves from its crown, with a pale vein each.
function leaves(ctx, m, amount) {
  const len = m.h * (0.2 + 0.16 * amount), half = len * 0.36;
  for (const side of [-1, 1]) {
    const x = m.cx + side * m.w * 0.05, y = m.minY + m.h * 0.06;
    const tipX = x + side * len * 0.75, tipY = y - len * 0.7;
    const g = ctx.createLinearGradient(x, y, tipX, tipY);
    g.addColorStop(0, 'rgba(60,150,80,.95)');
    g.addColorStop(1, 'rgba(160,235,120,.95)');
    const nx = side * 0.7, ny = 0.7;
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.quadraticCurveTo(x + side * len * 0.2 - nx * half, y - len * 0.5 - ny * half * 0.2, tipX, tipY);
    ctx.quadraticCurveTo(x + side * len * 0.65 + nx * half, y - len * 0.1 + ny * half * 0.3, x, y);
    ctx.closePath();
    ctx.fillStyle = g;
    ctx.fill();
    ctx.strokeStyle = 'rgba(235,255,225,.7)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.lineTo(tipX, tipY);
    ctx.stroke();
  }
}

// A starry slime has two small wings on its flanks, pale and a little translucent, each a plump leaf shape with a pale rib.
function wings(ctx, m, amount) {
  const rx = m.w * (0.17 + 0.07 * amount), ry = m.h * (0.1 + 0.05 * amount);
  for (const s of [-1, 1]) {
    const x = m.cx + s * (m.w * 0.42 + rx * 0.55), y = m.minY + m.h * 0.42;
    const g = ctx.createLinearGradient(x - s * rx, y + ry, x + s * rx, y - ry);
    g.addColorStop(0, 'rgba(240,248,255,.92)');
    g.addColorStop(1, 'rgba(185,210,255,.72)');
    ctx.beginPath();
    ctx.ellipse(x, y, rx, ry, -s * 0.55, 0, TAU);
    ctx.fillStyle = g;
    ctx.fill();
    ctx.strokeStyle = 'rgba(255,255,255,.85)';
    ctx.lineWidth = 1.2;
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(x - s * rx * 0.8 * Math.cos(0.55), y + rx * 0.8 * Math.sin(0.55));
    ctx.lineTo(x + s * rx * 0.8 * Math.cos(0.55), y - rx * 0.8 * Math.sin(0.55));
    ctx.strokeStyle = 'rgba(255,255,255,.5)';
    ctx.stroke();
  }
}

/** Draws the accessories grown by the genome's materials, in the body's own frame, behind the face. */
export function drawAccessories(ctx, m, layers, skin) {
  if (!layers) return;
  if (layers.fur >= SHOW_FROM) ears(ctx, m, layers.fur, skin);
  if (layers.crystal >= SHOW_FROM) crystals(ctx, m, layers.crystal);
  if (layers.honey >= SHOW_FROM) honeycomb(ctx, m, layers.honey);
  if (layers.stars >= SHOW_FROM) wings(ctx, m, layers.stars);
  if (layers.lava >= SHOW_FROM) horns(ctx, m, layers.lava);
  if (layers.stripes >= SHOW_FROM) leaves(ctx, m, layers.stripes);
}

// Limbs are extra polygons handed to the gel shader, which merges them smoothly with the body: they are
// real gel (translucent, lit, glossy), not stickers. All coordinates are in the body's own frame.
function ellipsePoly(cx, cy, rx, ry, n = 12, turn = 0) {
  const poly = Array.from({ length: n }, (_, k) => {
    const a = (k / n) * TAU;
    return [cx + Math.cos(a) * rx * Math.cos(turn) - Math.sin(a) * ry * Math.sin(turn), cy + Math.cos(a) * rx * Math.sin(turn) + Math.sin(a) * ry * Math.cos(turn)];
  });
  poly.thick = Math.min(rx, ry) * 2;
  return poly;
}

// A claw: a ball with a wedge cut out toward the outside.
function clawPoly(cx, cy, r, outward, n = 18) {
  const poly = Array.from({ length: n }, (_, k) => {
    const a = (k / n) * TAU, off = Math.abs(wrap(a - outward));
    const radius = off < 0.55 ? r * (0.3 + 0.7 * (off / 0.55) ** 1.5) : r;
    return [cx + Math.cos(a) * radius, cy + Math.sin(a) * radius];
  });
  poly.thick = r * 1.7;
  return poly;
}

const LIMBS = {
  crab: (m, t, push) => {
    const r = m.w * 0.12, sway = Math.sin(t * 1.4) * m.w * 0.012, polys = [];
    for (const side of [-1, 1]) {
      polys.push(clawPoly((side < 0 ? m.minX : m.maxX) + side * r * 0.3 + push.x * m.w * 0.05, m.minY + m.h * 0.5 + sway * side + push.y * m.h * 0.3, r, side < 0 ? Math.PI - 0.25 : 0.25));
    }
    const legW = m.w * 0.09, legH = m.h * 0.34;
    [0.2, 0.38, 0.62, 0.8].forEach((at, i) => polys.push(ellipsePoly(m.minX + m.w * at, m.maxY - legH * 0.55 + Math.sin(t * 2 + i * 1.7) * 0.8, legW * 0.55, legH * 0.55, 10)));
    return polys;
  },
  // The body arches into a tail; two plump flukes sit at its tip (gel is translucent, so thin parts would vanish).
  whale: (m, t, push) => {
    const flick = Math.sin(t * 1.8) * 0.06 + push.x * 0.5;
    const x = m.maxX - m.w * 0.06 + push.x * m.w * 0.12, y = m.minY + m.h * 0.14, s = m.w * 0.13;
    const fluke = (side) => ellipsePoly(x - m.w * 0.02 + side * s * 1.05, y - s * 0.2 - side * s * 0.15 * Math.sin(flick * 3), s * 1.15, s * 0.55, 14, -0.5 + side * (0.75 + flick));
    return [
      fluke(-1),
      fluke(1),
      ellipsePoly(m.cx + m.w * 0.1, m.maxY - m.h * 0.3, m.w * 0.13, m.h * 0.09, 12, 0.6 + flick),
    ];
  },
  // The saucer hat: a thick brim and a dome, sitting on the top of the body.
  ufo: (m, t, push) => [
    ellipsePoly(m.cx + push.x * m.w * 0.05, m.minY + m.h * 0.1 + push.y * m.h * 0.1, m.w * 0.62, m.h * 0.15, 18),
    ellipsePoly(m.cx + push.x * m.w * 0.08, m.minY - m.h * 0.12 + push.y * m.h * 0.15, m.w * 0.3, m.h * 0.2, 14),
  ],
  // The llama: a head forward of the neck with a muzzle, two short ears swept back, and a puff of wool.
  llama: (m, t, push) => {
    const hx = m.cx + m.w * 0.22 + push.x * m.w * 0.06, hy = m.minY + m.h * 0.02 + Math.sin(t * 1.6) * 0.8;
    return [
      ellipsePoly(hx, hy, m.w * 0.23, m.h * 0.15, 14, 0.15),
      ellipsePoly(hx + m.w * 0.17, hy + m.h * 0.04, m.w * 0.13, m.h * 0.09, 12, 0.1),
      ellipsePoly(hx - m.w * 0.09, hy - m.h * 0.15, m.w * 0.05, m.h * 0.14, 10, -0.5),
      ellipsePoly(hx + m.w * 0.02, hy - m.h * 0.16, m.w * 0.05, m.h * 0.14, 10, -0.2),
      ellipsePoly(m.cx - m.w * 0.05, m.minY + m.h * 0.12 + push.y * m.h * 0.1, m.w * 0.2, m.h * 0.12, 14),
    ];
  },
  // The bat: two broad wings that flap slowly, and two small ears.
  bat: (m, t, push) => {
    const flap = Math.sin(t * 2.2) * 0.12 + push.y * 0.6;
    const wings = [-1, 1].map((side) => ellipsePoly(m.cx + side * m.w * 0.58, m.minY + m.h * 0.42, m.w * 0.3, m.h * 0.2, 16, side * (-0.55 - flap) + push.x * 0.4));
    const ears = [-1, 1].map((side) => ellipsePoly(m.cx + side * m.w * 0.24, m.minY - m.h * 0.06, m.w * 0.07, m.h * 0.2, 12, side * 0.25));
    return wings.concat(ears);
  },
  // The rock: thick crystals jutting from the top.
  rock: (m, t, push) => [[-0.26, 0.1, 0.1, -0.25], [0, -0.1, 0.12, 0], [0.27, 0.05, 0.1, 0.25]]
    .map(([dx, dy, k, lean]) => ellipsePoly(m.cx + m.w * dx + push.x * m.w * 0.05, m.minY + m.h * dy + push.y * m.h * 0.1, m.w * k, m.h * 0.28, 4, lean)),
  cloud: (m, t, push) => [[-0.27, 0.16, 0.2], [0.02, 0.04, 0.24], [0.29, 0.16, 0.19]]
    .map(([dx, dy, k], i) => ellipsePoly(m.cx + m.w * dx + push.x * m.w * 0.04 * (1 + i * 0.4), m.minY + m.h * dy + push.y * m.h * 0.2 * (1 + i * 0.3) + Math.sin(t * 1.2 + i * 2) * 0.8, m.w * k, m.w * k, 14)),
};

/** The limb polygons of a form (each an array of [x, y]) for the gel shader; none for the plain slime. */
export function limbPolygons(name, m, t, sway = { x: 0, y: 0 }) {
  return LIMBS[name] ? LIMBS[name](m, t, sway) : [];
}

// What stays flat on the body: the whale's pale belly and the cloud's prompt.
function whaleBelly(ctx, m, palette) {
  ctx.fillStyle = rgba(palette.light, 0.26);
  ctx.beginPath();
  ctx.ellipse(m.cx - m.w * 0.05, m.maxY - m.h * 0.17, m.w * 0.36, m.h * 0.13, 0, 0, TAU);
  ctx.fill();
}

function cloudPrompt(ctx, m) {
  const x = m.cx - m.w * 0.17, y = m.maxY - m.h * 0.24, k = m.w * 0.06;
  ctx.strokeStyle = 'rgba(255,255,255,.88)';
  ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  ctx.lineWidth = Math.max(2, m.w * 0.022);
  ctx.beginPath();
  ctx.moveTo(x, y - k); ctx.lineTo(x + k * 0.9, y); ctx.lineTo(x, y + k);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(x + k * 1.7, y + k); ctx.lineTo(x + k * 3.2, y + k);
  ctx.stroke();
}

/** Draws the flat marks of a form, in the body's own frame. */
export function drawFormParts(ctx, name, m, palette) {
  if (name === 'cloud') cloudPrompt(ctx, m);
}

/** True when (x, y) is inside a polygon given as an array of [x, y]. */
export function insidePolygon(poly, x, y) {
  let inside = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const [xi, yi] = poly[i], [xj, yj] = poly[j];
    if ((yi > y) !== (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}

/** The same test for a flat [x0, y0, x1, y1, ...] outline. */
export function insideFlat(flat, x, y) {
  const poly = [];
  for (let i = 0; i < flat.length; i += 2) poly.push([flat[i], flat[i + 1]]);
  return insidePolygon(poly, x, y);
}

/** The outline point nearest to (x, y), pulled `inward` px toward the centroid: where a grab on a limb lands in the body. */
export function bodyPointNear(flat, x, y, inward = 12) {
  let best = 0, bestD = Infinity, cx = 0, cy = 0;
  for (let i = 0; i < flat.length; i += 2) {
    cx += flat[i]; cy += flat[i + 1];
    const d = (flat[i] - x) ** 2 + (flat[i + 1] - y) ** 2;
    if (d < bestD) { bestD = d; best = i; }
  }
  cx /= flat.length / 2; cy /= flat.length / 2;
  const dx = cx - flat[best], dy = cy - flat[best + 1], len = Math.hypot(dx, dy) || 1;
  return { x: flat[best] + (dx / len) * inward, y: flat[best + 1] + (dy / len) * inward };
}

/** Half the width of a form's resting body in radii (claws and wings included, at least 1): how far from a cursor it must stand to let go of it. */
export function formReach(name) {
  const values = profile(name);
  let widest = 1;
  for (let i = 0; i < values.length; i++) {
    const angle = (i / values.length) * Math.PI * 2;
    if (Math.abs(Math.sin(angle)) < 0.6) widest = Math.max(widest, values[i] * Math.abs(Math.cos(angle)));
  }
  return widest;
}
