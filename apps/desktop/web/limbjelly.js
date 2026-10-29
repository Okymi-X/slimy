// A soft layer over the limb polygons: every vertex is a little spring that lags the body's acceleration and
// is pushed away by the pointer, so a tail or a claw wobbles and dents like gel instead of moving as one piece.
// Everything is in the body's own frame. Vertices near the body are stiff, the tips are free.

const STIFFNESS = 85;
const DAMPING = 8.5;
const INERTIA = 0.0022;
const PUSH_RADIUS = 0.2;
const PUSH_FORCE = 1500;
const MAX_OFFSET = 0.13;
const FREE_OVER = 0.14;
// While a limb is held: how hard the gel near the held point follows the pointer, over what distance, and how far it may stretch.
const HOLD_PULL = 1100;
const HOLD_DRAG = 38;
const HOLD_RADIUS = 0.3;
const HOLD_STRETCH = 1.4;
// Longest sub-step and longest time one call may cover (the page never asks for more than 0.25 s).
const SUB_STEP = 1 / 90;
const MAX_DT = 0.25;
// When a hold ends, an offset beyond the resting limit shrinks at this rate instead of being cut at once.
const RELAX = 7;

export function createLimbJelly() {
  let states = [];

  function sync(polys) {
    if (states.length !== polys.length || states.some((s, i) => s.length !== polys[i].length)) {
      states = polys.map((poly) => poly.map(() => ({ x: 0, y: 0, vx: 0, vy: 0 })));
    }
  }

  return {
    /**
     * Moves the vertices of `polys` (arrays of [x, y]) in place.
     * body: the box {minX, maxX, minY, maxY, w}; acc: body-frame acceleration in px/s^2;
     * pointer: {x, y} in the body frame, or null when it is not touching; dt in seconds.
     * hold: {poly, vertex, to: [x, y]} while a limb is held: that vertex sits exactly on `to`, the gel around it follows.
     * All the elapsed time is used, in bounded sub-steps, so the motion does not depend on the frame rate.
     */
    apply(polys, { body, acc, pointer, dt, hold = null }) {
      sync(polys);
      const total = Math.min(Math.max(dt, 0), MAX_DT);
      const steps = Math.max(1, Math.ceil(total / SUB_STEP)), h = total / steps;
      const limit = body.w * (hold ? HOLD_STRETCH : MAX_OFFSET), radius = body.w * PUSH_RADIUS;
      const heldPoint = hold && polys[hold.poly] && polys[hold.poly][hold.vertex];
      const anchor = heldPoint ? [heldPoint[0], heldPoint[1]] : null;
      const freedom = (x, y) => Math.min(1, 0.2 + Math.max(body.minX - x, x - body.maxX, body.minY - y, y - body.maxY, 0) / (body.w * FREE_OVER));
      const anchorFree = anchor ? freedom(anchor[0], anchor[1]) : 1;
      const reach = body.w * HOLD_RADIUS;
      const rest = polys.map((poly) => poly.map((p) => [p[0], p[1]]));
      for (let step = 0; step < steps; step++) {
        rest.forEach((poly, k) => {
          poly.forEach((point, i) => {
            const s = states[k][i];
            if (anchor && k === hold.poly && i === hold.vertex) {
              // The held vertex is the pointer's: no spring may keep it away.
              s.x = hold.to[0] - point[0]; s.y = hold.to[1] - point[1]; s.vx = 0; s.vy = 0;
              return;
            }
            const free = freedom(point[0], point[1]);
            let fx = -STIFFNESS * s.x - DAMPING * s.vx - acc.x * INERTIA * free;
            let fy = -STIFFNESS * s.y - DAMPING * s.vy - acc.y * INERTIA * free;
            let drag = 0;
            if (anchor) {
              const gap = Math.hypot(point[0] - anchor[0], point[1] - anchor[1]);
              const weight = Math.exp(-((gap / reach) ** 2)) * Math.min(1, free / anchorFree) ** 2;
              if (weight > 0.01) {
                drag = HOLD_DRAG * weight;
                fx += (hold.to[0] - point[0] - s.x) * HOLD_PULL * weight;
                fy += (hold.to[1] - point[1] - s.y) * HOLD_PULL * weight;
              }
            } else if (pointer) {
              const dx = point[0] + s.x - pointer.x, dy = point[1] + s.y - pointer.y, d = Math.hypot(dx, dy);
              if (d < radius && d > 0.01) {
                const push = (1 - d / radius) * PUSH_FORCE * free / d;
                fx += dx * push; fy += dy * push;
              }
            }
            s.vx += fx * h; s.vy += fy * h;
            if (drag > 0) { const keep = Math.exp(-drag * h); s.vx *= keep; s.vy *= keep; }
            s.x += s.vx * h; s.y += s.vy * h;
            // Past the limit an offset relaxes toward it instead of being cut: a limb let go after a stretch must not snap.
            const len = Math.hypot(s.x, s.y);
            if (len > limit) {
              // Strict while a limb is held (that is the stretch limit); relaxed once it is let go.
              const k = (hold ? limit : Math.max(limit, len * Math.exp(-RELAX * h))) / len;
              s.x *= k; s.y *= k;
            }
          });
        });
      }
      polys.forEach((poly, k) => poly.forEach((point, i) => { point[0] += states[k][i].x; point[1] += states[k][i].y; }));
    },
    reset() { states = []; },
  };
}

/** The vertex nearest to (x, y) among displayed polygons (arrays of [x, y]): {poly, vertex, d}, or null when there are none. */
export function pickVertex(polys, x, y) {
  let best = null;
  polys.forEach((poly, k) => poly.forEach(([px, py], i) => {
    const d = Math.hypot(px - x, py - y);
    if (!best || d < best.d) best = { poly: k, vertex: i, d };
  }));
  return best;
}
