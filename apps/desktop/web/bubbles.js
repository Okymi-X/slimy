// Bubbles trapped in the gel. They live in the body frame (x right, y down, origin at the
// centroid), rise toward the top at a speed that grows with their size, wobble, lag behind when
// the body accelerates, stay inside the outline and pop at the top before a new one forms.

export const BUBBLE_COUNT = 12;

const TOP_ZONE = 0.16;
const POP_TIME = 0.14;
const FORM_TIME = 0.5;

function size() {
  const roll = Math.random();
  if (roll < 0.55) return 2.8 + Math.random() * 2.6;
  if (roll < 0.88) return 6 + Math.random() * 3.5;
  return 10.5 + Math.random() * 5;
}

function fresh(box, now) {
  const r = size();
  return {
    x: box.x0 + (box.x1 - box.x0) * (0.2 + 0.6 * Math.random()),
    y: box.y1 - r - 3 - Math.random() * (box.y1 - box.y0) * 0.5,
    vx: 0, vy: 0,
    r,
    depth: 0.45 + 0.55 * Math.random(),
    phase: Math.random() * 6.28,
    born: now,
    state: 'forming',
    timer: 0,
  };
}

export function createBubbles({ count = BUBBLE_COUNT, scale: sizeScale = 1, rise: riseScale = 1 } = {}) {
  const list = [];
  const active = Math.min(count, BUBBLE_COUNT);

  return {
    /**
     * env: { box: {x0, y0, x1, y1} body bounds in the body frame, acc: {x, y} body acceleration in that
     * frame, scale, squash }. Returns [{x, y, r, a}] in the body frame.
     */
    step(dt, now, env) {
      const { box, acc, scale, squash } = env;
      const midX = (box.x0 + box.x1) / 2, midY = (box.y0 + box.y1) / 2;
      const rx = (box.x1 - box.x0) / 2, ry = (box.y1 - box.y0) / 2;
      while (list.length < BUBBLE_COUNT) list.push({ ...fresh(box, now), state: 'waiting', timer: Math.random() * 2 });

      return list.map((b, i) => {
        if (i >= active) return { x: 0, y: 0, r: 0, a: 0 };
        if (b.state === 'waiting') {
          b.timer -= dt;
          if (b.timer <= 0) list[i] = Object.assign(b, fresh(box, now));
          return { x: 0, y: 0, r: 0, a: 0 };
        }

        const rise = (6 + b.r * 3.2) * b.depth * riseScale;
        b.vx += (-acc.x * 0.3) * dt;
        b.vy += (-acc.y * 0.3 - (b.vy + rise) * 2.2) * dt;
        b.vx *= Math.exp(-3 * dt);
        b.x += (b.vx + Math.sin(now * 1.7 + b.phase) * 4 * b.depth) * dt;
        b.y += b.vy * dt;

        const nx = (b.x - midX) / (rx * 0.78), ny = (b.y - midY) / (ry * 0.78);
        const edge = Math.hypot(nx, ny);
        if (edge > 1) {
          b.x = midX + (nx / edge) * rx * 0.78;
          b.y = midY + (ny / edge) * ry * 0.78;
          b.vx *= -0.3;
          b.vy = Math.max(b.vy, 0) * -0.3;
        }

        let fade = 1;
        if (b.state === 'forming') {
          b.timer += dt;
          fade = Math.min(b.timer / FORM_TIME, 1);
          if (fade >= 1) b.state = 'rising';
        }
        if (b.state === 'rising' && b.y < box.y0 + (box.y1 - box.y0) * TOP_ZONE + b.r) {
          b.state = 'popping';
          b.timer = 0;
        }
        let grow = 1;
        if (b.state === 'popping') {
          b.timer += dt;
          const k = Math.min(b.timer / POP_TIME, 1);
          fade = 1 - k;
          grow = 1 + 0.35 * k;
          if (k >= 1) {
            b.state = 'waiting';
            b.timer = 0.4 + Math.random() * 2.5;
          }
        }
        return { x: b.x, y: b.y, r: b.r * sizeScale * scale * grow * Math.min(1, squash + 0.25), a: fade * (0.5 + 0.5 * b.depth) };
      });
    },
  };
}
