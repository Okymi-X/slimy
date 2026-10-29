// Frame orchestration: smooths the simulated outline, keeps the purely visual state (core lag,
// bubbles, palette blend), repaints a small sprite that the page moves with the slime, and layers the face over the gel.

import { createGel, PALETTES } from './gel.js';
import { createBubbles } from './bubbles.js';
import { createBurst, drawEgg } from './egg.js';
import { createLimbJelly, pickVertex } from './limbjelly.js';
import { drawAccessories, drawFormParts, limbPolygons, insidePolygon } from './forms.js';
import { createFace } from './face.js';
import { createSwallowed } from './swallowed.js';

const N = 56;
const PAD = 20;
export const SPRITE = 400;

function smoothOutline(pts) {
  const soft = new Float32Array(N * 2);
  for (let i = 0; i < N; i++) {
    const a = ((i + N - 1) % N) * 2, b = i * 2, c = ((i + 1) % N) * 2;
    soft[b] = (pts[a] + 2 * pts[b] + pts[c]) / 4;
    soft[b + 1] = (pts[a + 1] + 2 * pts[b + 1] + pts[c + 1]) / 4;
  }
  return soft;
}

function rotateAround(x, y, c, angle) {
  const cos = Math.cos(angle), sin = Math.sin(angle), dx = x - c.x, dy = y - c.y;
  return { x: c.x + dx * cos - dy * sin, y: c.y + dx * sin + dy * cos };
}

function measure(pts, c, phi, restHeight) {
  let minX = 1e9, maxX = -1e9, minY = 1e9, maxY = -1e9;
  for (let i = 0; i < N; i++) {
    const p = rotateAround(pts[i * 2], pts[i * 2 + 1], c, -phi);
    minX = Math.min(minX, p.x); maxX = Math.max(maxX, p.x);
    minY = Math.min(minY, p.y); maxY = Math.max(maxY, p.y);
  }
  const w = maxX - minX, h = maxY - minY;
  return { cx: c.x, cy: c.y, minX, maxX, minY, maxY, w, h, squash: h / restHeight };
}

export function contactPatch(pts, up, gap) {
  const tx = -up.y, ty = up.x;
  let bottom = Infinity;
  for (let i = 0; i < pts.length; i += 2) bottom = Math.min(bottom, pts[i] * up.x + pts[i + 1] * up.y);
  let left = Infinity, right = -Infinity;
  for (let i = 0; i < pts.length; i += 2) {
    if (pts[i] * up.x + pts[i + 1] * up.y > bottom + 5) continue;
    const across = pts[i] * tx + pts[i + 1] * ty;
    left = Math.min(left, across); right = Math.max(right, across);
  }
  const middle = (left + right) / 2, plane = bottom - gap;
  return { x: tx * middle + up.x * plane, y: ty * middle + up.y * plane, width: Math.max(4, (right - left) / 2) };
}

function worldBox(outline) {
  let x0 = 1e9, y0 = 1e9, x1 = -1e9, y1 = -1e9;
  for (let i = 0; i < outline.length; i += 2) {
    x0 = Math.min(x0, outline[i]); x1 = Math.max(x1, outline[i]);
    y0 = Math.min(y0, outline[i + 1]); y1 = Math.max(y1, outline[i + 1]);
  }
  return { x0: x0 - PAD, y0: y0 - PAD, x1: x1 + PAD, y1: y1 + PAD };
}

export function createRenderer({ glCanvas, overCanvas, dpr, model }) {
  const gel = createGel(glCanvas);
  const over = overCanvas.getContext('2d');
  const face = createFace(over, model);
  const { material } = model;

  const palette = Object.fromEntries(Object.entries(material.colors).map(([k, v]) => [k, [...v]]));
  let target = material.colors;
  const core = { x: 0, y: 0, vx: 0, vy: 0, ready: false };
  const bubbles = createBubbles(model.bubbles);
  const burst = createBurst();
  const swallowed = createSwallowed();
  const motion = { x: 0, y: 0, vx: 0, vy: 0, ready: false };

  let dull = 0;
  // Limbs swing with the body: a spring pair per axis, driven by the body's acceleration.
  const swing = { x: 0, vx: 0, y: 0, vy: 0 };
  let worldLimbs = [];
  const jelly = createLimbJelly();
  let held = null;
  let lastAcc = { x: 0, y: 0 };
  function blendPalette(dt) {
    const f = 1 - Math.exp(-4 * dt);
    for (const k in palette) {
      const [r, g, b] = target[k], grey = 0.3 * r + 0.59 * g + 0.11 * b;
      const goal = [r, g, b].map((v) => v + (grey - v) * dull * 0.65);
      palette[k] = palette[k].map((v, i) => v + (goal[i] - v) * f);
    }
  }

  function updateCore(m, radius, dt) {
    if (!core.ready) { core.x = m.cx; core.y = m.cy; core.ready = true; }
    core.vx += (m.cx - core.x) * 38 * dt;
    core.vy += (m.cy + radius * 0.12 - core.y) * 38 * dt;
    const keep = Math.exp(-7 * dt);
    core.vx *= keep; core.vy *= keep;
    core.x += core.vx * dt * 3.6;
    core.y += core.vy * dt * 3.6;
  }

  return {
    /** The two canvases are a small sprite that the page moves over the transparent window. */
    init() {
      gel.resize(SPRITE, SPRITE, dpr);
      overCanvas.width = Math.round(SPRITE * dpr);
      overCanvas.height = Math.round(SPRITE * dpr);
    },
    /** True while something on the face is moving, so the page keeps a smooth frame rate. */
    busy() { return face.busy() || burst.active; },
    setColors(colors) { target = colors; },
    hint(name, seconds) { face.hint(name, seconds); },
    setInk(color) { face.setInk(color); },
    /** 0 to 1: how washed out the gel looks (a sulking slime loses its colors). */
    setDull(amount) { dull = amount; },
    burst(x, y) { burst.emit(x, y); },
    /** True when (x, y) is on a limb drawn last frame. */
    limbContains(x, y) { return worldLimbs.some((poly) => insidePolygon(poly, x, y)); },
    /** Bounding box of the limbs drawn last frame, or null: the touchable area beyond the body. */
    limbBox() {
      if (!worldLimbs.length) return null;
      let x0 = 1e9, y0 = 1e9, x1 = -1e9, y1 = -1e9;
      for (const poly of worldLimbs) for (const [x, y] of poly) { x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y); }
      return { x0, y0, x1, y1 };
    },
    setPalette(name) { target = PALETTES[name] ?? target; },
    /** frame: { pts, restHeight, radius, gap, gravityUp, t, dt, pointer: {x, y, hover, known}, sleepiness, grabbed, orientation, view } in world px; view is the sprite origin. */
    draw(frame) {
      const { pts, view, orientation: phi, t, dt } = frame;
      const viewSize = { w: SPRITE, h: SPRITE };
      let cx = 0, cy = 0;
      for (let i = 0; i < N; i++) { cx += pts[i * 2]; cy += pts[i * 2 + 1]; }
      const c = { x: cx / N, y: cy / N };
      const m = measure(pts, c, phi, frame.restHeight);
      blendPalette(dt);
      updateCore(m, frame.radius, dt);

      const toWorld = (x, y) => rotateAround(x, y, c, phi);
      // The body outline first, then the form's limbs as extra polygons the shader merges into one gel.
      let outline = smoothOutline(pts);
      let ends = null;
      let polyThick = null;
      const limbs = frame.form ? limbPolygons(frame.form, m, t, swing) : [];
      const displayed = worldLimbs;
      worldLimbs = [];
      if (limbs.length) {
        const local = rotateAround(frame.pointer.x, frame.pointer.y, c, -phi);
        // A press that started on a limb holds the vertex nearest to the pointer among the limbs as they were
        // displayed (deformed), and keeps its indices. A reference to a vertex that no longer exists is dropped.
        if (frame.limbGrab && !held) {
          const best = pickVertex(displayed, frame.pointer.x, frame.pointer.y);
          held = best && limbs[best.poly] && limbs[best.poly][best.vertex] ? best : null;
        }
        if (!frame.limbGrab || (held && !(limbs[held.poly] && limbs[held.poly][held.vertex]))) held = null;
        const touching = frame.pointer.known && frame.pointer.hover && !frame.grabbed && !held;
        jelly.apply(limbs, { body: m, acc: lastAcc, pointer: touching ? local : null, dt, hold: held ? { poly: held.poly, vertex: held.vertex, to: [local.x, local.y] } : null });
      } else {
        jelly.reset();
        held = null;
      }
      if (limbs.length) {
        const flat = [...outline];
        ends = [N];
        polyThick = [0];
        for (const poly of limbs) {
          const placed = [];
          for (const [x, y] of poly) { const w = toWorld(x, y); flat.push(w.x, w.y); placed.push([w.x, w.y]); }
          worldLimbs.push(placed);
          ends.push(flat.length / 2);
          polyThick.push((poly.thick ?? Math.min(m.w, m.h) * 0.3) * material.thickness);
        }
        outline = Float32Array.from(flat);
      }
      const quad = worldBox(outline);

      const coreWorld = toWorld(core.x, core.y);
      const step = Math.max(dt, 1e-3);
      const vx = motion.ready ? (c.x - motion.x) / step : 0, vy = motion.ready ? (c.y - motion.y) / step : 0;
      const ax = Math.max(-5000, Math.min(5000, (vx - motion.vx) / step)), ay = Math.max(-5000, Math.min(5000, (vy - motion.vy) / step));
      Object.assign(motion, { x: c.x, y: c.y, vx, vy, ready: true });
      const cos = Math.cos(-phi), sin = Math.sin(-phi);
      const bodyAcc = { x: ax * cos - ay * sin, y: ax * sin + ay * cos };
      lastAcc = bodyAcc;
      {
        // All the elapsed time is used, in bounded sub-steps: the swing must not depend on the frame rate.
        const total = Math.min(step, 0.25), n = Math.max(1, Math.ceil(total / (1 / 90))), h = total / n;
        const push = (acc, k, c, gain, state, v) => { state[v] += (-k * state[acc] - c * state[v] - gain * Math.max(-3000, Math.min(3000, acc === 'x' ? bodyAcc.x : bodyAcc.y))) * h; state[acc] = Math.max(-0.9, Math.min(0.9, state[acc] + state[v] * h)); };
        for (let i = 0; i < n; i++) {
          push('x', 55, 6.5, 0.0004, swing, 'vx');
          push('y', 70, 7.5, 0.0003, swing, 'vy');
        }
      }
      const bodyVel = { x: vx * cos - vy * sin, y: vx * sin + vy * cos };
      const live = bubbles.step(dt, t, {
        box: { x0: m.minX - c.x, y0: m.minY - c.y, x1: m.maxX - c.x, y1: m.maxY - c.y },
        acc: bodyAcc,
        scale: frame.radius / 64,
        squash: m.squash,
      }).map((b) => ({ ...toWorld(c.x + b.x, c.y + b.y), r: b.r, a: b.a }));

      gel.draw({
        view, viewSize, dpr, quad,
        outline, count: outline.length / 2, ends, polyThick,
        center: c,
        up: frame.gravityUp,
        bodyUp: { x: Math.sin(phi), y: -Math.cos(phi) },
        support: contactPatch(pts, frame.gravityUp, frame.gap),
        half: { x: m.w / 2, y: m.h / 2 },
        thick: Math.min(m.w, m.h) * material.thickness,
        squash: m.squash,
        time: t,
        contact: Math.max(0, Math.min(1, 1 - frame.gap / 22)),
        core: coreWorld,
        coreRadius: frame.radius * material.coreRadius,
        palette, material, bubbles: live,
      });

      over.setTransform(dpr, 0, 0, dpr, -view.x * dpr, -view.y * dpr);
      over.clearRect(view.x, view.y, SPRITE, SPRITE);
      over.save();
      over.translate(c.x, c.y); over.rotate(phi); over.translate(-c.x, -c.y);
      const local = rotateAround(frame.pointer.x, frame.pointer.y, c, -phi);
      if (frame.egg) {
        drawEgg(over, m, frame.egg.cracks);
      } else if ((frame.awake ?? 1) > 0.02) {
        const eyeLevel = { x: m.cx, y: m.minY + m.h * model.face.eyes.height };
        const look = frame.lookOverride ?? (frame.pointer.known
          ? { x: Math.tanh((local.x - eyeLevel.x) / 180), y: Math.tanh((local.y - eyeLevel.y) / 140) }
          : null);
        drawFormParts(over, frame.form, m, palette);
        drawAccessories(over, m, material.layers, palette.mid);
        swallowed.step(dt, frame.catchPhase);
        const inside = rotateAround(coreWorld.x, coreWorld.y, c, -phi);
        swallowed.draw(over, inside.x, inside.y, frame.radius, t);
        face.draw(m, {
          t, dt, look, vel: bodyVel, acc: bodyAcc,
          sleepiness: frame.sleepiness, grabbed: frame.grabbed, hover: frame.pointer.hover && frame.pointer.active, pointerSpeed: frame.pointer.speed,
          speed: Math.hypot(vx, vy), airborne: frame.gap > 8, awake: frame.awake ?? 1, sulking: !!frame.sulking,
          impact: frame.impact ?? 0,
          intent: frame.intent ?? 'idle', catchPhase: frame.catchPhase, crouching: !!frame.crouching,
          voice: frame.voice ?? 0,
        });
      }
      over.restore();
      burst.draw(over, dt);
    },
  };
}
