// The cursor, briefly visible inside the gel while the slime holds it. Pure rendering: a visibility level that rises
// while the catch holds the cursor and fades out when it lets go, and a small arrow silhouette drawn in the core.
export const HELD_PHASES = ['contact', 'spread', 'hold'];
export const RISE_SECONDS = 0.25;
export const FADE_SECONDS = 1.2;

/** Next visibility (0 to 1) after dt seconds, given the catch phase. */
export function swallowedLevel(level, dt, phase) {
  const held = HELD_PHASES.includes(phase);
  const step = dt / (held ? RISE_SECONDS : FADE_SECONDS);
  return held ? Math.min(1, level + step) : Math.max(0, level - step);
}

export function createSwallowed() {
  let level = 0;
  return {
    step(dt, phase) { level = swallowedLevel(level, Math.max(0, Math.min(dt, 0.25)), phase); return level; },
    get level() { return level; },
    /** Draws the arrow at (x, y), scaled by the body radius, bobbing slightly as if floating in the gel. */
    draw(ctx, x, y, radius, t) {
      if (level < 0.01) return;
      // Sits just below the face (under the mouth), smaller than the body so it never covers the features.
      const s = (radius / 64) * 0.8 * (0.8 + 0.2 * level), bob = Math.sin(t * 2.2) * 1.5;
      ctx.save();
      ctx.translate(x, y + radius * 0.1 + bob);
      ctx.rotate(-0.15 + Math.sin(t * 1.6) * 0.05);
      ctx.scale(s, s);
      ctx.globalAlpha = 0.55 * level;
      ctx.beginPath();
      ctx.moveTo(-6, -11); ctx.lineTo(-6, 9); ctx.lineTo(-1.5, 5); ctx.lineTo(1.5, 12); ctx.lineTo(4.5, 10.5); ctx.lineTo(1.5, 4); ctx.lineTo(7.5, 4);
      ctx.closePath();
      ctx.fillStyle = 'rgba(255,255,255,0.9)'; ctx.fill();
      ctx.lineWidth = 1.4; ctx.strokeStyle = 'rgba(20,40,40,0.7)'; ctx.stroke();
      ctx.restore();
    },
  };
}
