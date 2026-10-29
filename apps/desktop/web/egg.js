// The egg: cracks that grow with every touch, and the burst of sparkles when it hatches.

const TAU = Math.PI * 2;

// Crack paths in body units: x from the centre (fraction of width), y from the top (fraction of height).
const CRACKS = [
  [[0.04, 0.14], [-0.04, 0.24], [0.06, 0.32], [-0.03, 0.44]],
  [[-0.3, 0.36], [-0.18, 0.46], [-0.22, 0.56], [-0.08, 0.64]],
  [[0.3, 0.4], [0.2, 0.5], [0.26, 0.6], [0.12, 0.72]],
];

export function drawEgg(ctx, m, cracks) {
  const at = ([u, v]) => [m.cx + u * m.w, m.minY + v * m.h];
  ctx.save();
  ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  ctx.fillStyle = 'rgba(120,85,50,.28)';
  for (const [u, v, r] of [[-0.18, 0.3, 2.2], [0.2, 0.28, 1.8], [-0.05, 0.62, 2], [0.24, 0.66, 1.6], [-0.28, 0.58, 1.5]]) {
    const [x, y] = at([u, v]);
    ctx.beginPath(); ctx.arc(x, y, r, 0, TAU); ctx.fill();
  }
  for (let i = 0; i < Math.min(cracks, CRACKS.length); i++) {
    const path = CRACKS[i].map(at);
    for (const [color, width] of [['rgba(70,45,25,.8)', 2.6], ['rgba(255,245,220,.85)', 1]]) {
      ctx.strokeStyle = color; ctx.lineWidth = width;
      ctx.beginPath();
      path.forEach(([x, y], k) => (k === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y)));
      ctx.stroke();
    }
  }
  ctx.restore();
}

export function createBurst() {
  const sparks = [];
  const shards = [];
  return {
    emit(x, y) {
      for (let i = 0; i < 9; i++) {
        const angle = -Math.PI / 2 + (Math.random() - 0.5) * 2.4, speed = 140 + Math.random() * 260;
        shards.push({ x, y, vx: Math.cos(angle) * speed, vy: Math.sin(angle) * speed, spin: (Math.random() - 0.5) * 12, rot: Math.random() * TAU, age: 0, life: 1.5 + Math.random() * 0.6, size: 6 + Math.random() * 8 });
      }
      for (let i = 0; i < 26; i++) {
        const angle = Math.random() * TAU, speed = 60 + Math.random() * 220;
        sparks.push({ x, y, vx: Math.cos(angle) * speed, vy: Math.sin(angle) * speed - 80, age: 0, life: 0.7 + Math.random() * 0.6, size: 2 + Math.random() * 3 });
      }
    },
    draw(ctx, dt) {
      for (let i = shards.length - 1; i >= 0; i--) {
        const h = shards[i];
        h.age += dt;
        if (h.age > h.life) { shards.splice(i, 1); continue; }
        h.vy += 900 * dt; h.x += h.vx * dt; h.y += h.vy * dt; h.rot += h.spin * dt;
        const a = Math.min(1, (h.life - h.age) * 3);
        ctx.save();
        ctx.translate(h.x, h.y); ctx.rotate(h.rot); ctx.globalAlpha = a;
        ctx.fillStyle = '#f3e3c3'; ctx.strokeStyle = 'rgba(120,85,50,.8)'; ctx.lineWidth = 1.4; ctx.lineJoin = 'round';
        ctx.beginPath(); ctx.moveTo(-h.size * 0.6, h.size * 0.4); ctx.lineTo(0, -h.size * 0.6); ctx.lineTo(h.size * 0.7, h.size * 0.3); ctx.closePath();
        ctx.fill(); ctx.stroke();
        ctx.restore();
      }
      for (let i = sparks.length - 1; i >= 0; i--) {
        const s = sparks[i];
        s.age += dt;
        if (s.age > s.life) { sparks.splice(i, 1); continue; }
        s.vy += 380 * dt; s.x += s.vx * dt; s.y += s.vy * dt;
        const a = 1 - s.age / s.life;
        ctx.fillStyle = `rgba(255,${Math.round(220 - 60 * (1 - a))},170,${a})`;
        ctx.beginPath(); ctx.arc(s.x, s.y, s.size * a, 0, TAU); ctx.fill();
      }
    },
    get active() { return sparks.length > 0 || shards.length > 0; },
  };
}
