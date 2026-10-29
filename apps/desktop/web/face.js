// Face rig drawn on a 2D overlay above the gel, in the body frame (the caller rotates the context).
// The eyes and the mouth float in the gel on springs: they look at the pointer, trail behind when
// the body moves fast, slosh when it accelerates. The expression machine picks the pose.

import { createExpressions } from './expressions.js';

const TAU = Math.PI * 2;
const MAX_STEP = 1 / 120;
const POP_SPEED = 10;

const spring = () => ({ x: 0, y: 0, vx: 0, vy: 0 });
const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

function integrate(s, target, push, k, c, dt) {
  const steps = Math.max(1, Math.ceil(dt / MAX_STEP)), h = dt / steps;
  for (let i = 0; i < steps; i++) {
    s.vx += (k * (target.x - s.x) - c * s.vx + push.x) * h;
    s.vy += (k * (target.y - s.y) - c * s.vy + push.y) * h;
    s.x += s.vx * h;
    s.y += s.vy * h;
  }
}

function heart(ctx, x, y, size) {
  ctx.beginPath();
  ctx.moveTo(x, y + size * 0.9);
  ctx.bezierCurveTo(x - size * 1.5, y - size * 0.1, x - size * 0.7, y - size * 1.1, x, y - size * 0.35);
  ctx.bezierCurveTo(x + size * 0.7, y - size * 1.1, x + size * 1.5, y - size * 0.1, x, y + size * 0.9);
  ctx.fill();
}

export function createFace(ctx, config) {
  const { eyes, mouth, blush, look, lag, blink } = config.face;
  const idleGaze = config.motion.idleGaze;
  const baseInk = eyes.color;
  let ink = baseInk;
  const center = spring(), left = spring(), right = spring(), lips = spring();
  const gaze = { x: 0, y: 0 };
  const wander = { x: 0, y: 0, until: 0 };
  const machine = createExpressions();
  const shown = { eye: 'oval', mouth: 'smile', eyeScale: 1, mouthScale: 1 };
  // How open the mouth is while the slime speaks: follows the loudness of its voice, smoothed so it never flickers.
  let talk = 0;
  const floaters = [];
  let nextBlink = 2, blinkStart = -1, nextFloater = 0, moving = false, hint = null, now = 0;

  function aim(input, t, dt) {
    let target = input.look;
    if (!target && idleGaze) {
      if (t > wander.until) {
        wander.x = (Math.random() * 2 - 1) * 0.75;
        wander.y = (Math.random() * 2 - 1) * 0.45;
        wander.until = t + 1.5 + Math.random() * 3.5;
      }
      target = wander;
    }
    target = target ?? { x: 0, y: 0 };
    const ease = 1 - Math.exp(-look.follow * dt);
    gaze.x += (target.x - gaze.x) * ease;
    gaze.y += (target.y - gaze.y) * ease;
  }

  function blinkScale(t, sleepiness) {
    if (blinkStart < 0 && t > nextBlink) blinkStart = t;
    if (blinkStart < 0) return 1;
    const k = (t - blinkStart) / (0.16 + 0.12 * sleepiness);
    if (k >= 1) {
      blinkStart = -1;
      nextBlink = t + (blink.min + Math.random() * (blink.max - blink.min)) * (1 + sleepiness);
      return 1;
    }
    return 0.08 + 0.92 * Math.cos(Math.PI * k) ** 2;
  }

  // Eye and mouth kinds change through a quick shrink and regrow, so the swap is never seen.
  function pop(kind, wanted, key, dt) {
    const scaleKey = key === 'eye' ? 'eyeScale' : 'mouthScale';
    if (shown[key] !== wanted) {
      shown[scaleKey] -= dt * POP_SPEED;
      if (shown[scaleKey] <= 0) { shown[key] = wanted; shown[scaleKey] = 0; }
    } else {
      shown[scaleKey] = Math.min(1, shown[scaleKey] + dt * POP_SPEED);
    }
    return kind;
  }

  function drawEye(kind, x, y, rx, ry, pose, side, t, look2, blinking) {
    ctx.save();
    ctx.translate(x, y);
    ctx.scale(1, shown.eyeScale);
    ctx.rotate(side * pose.tilt);
    ctx.lineCap = 'round'; ctx.lineJoin = 'round';
    if (kind === 'heart') {
      ctx.fillStyle = '#ff4f7b';
      heart(ctx, 0, 0, rx * 1.1 * pose.wide);
      ctx.fillStyle = 'rgba(255,255,255,.85)';
      ctx.beginPath(); ctx.arc(-rx * 0.45, -rx * 0.4, rx * 0.25, 0, TAU); ctx.fill();
    } else if (kind === 'spiral') {
      ctx.strokeStyle = ink; ctx.lineWidth = 2.4;
      ctx.beginPath();
      for (let a = 0; a < TAU * 2.6; a += 0.3) {
        const r = (a / (TAU * 2.6)) * rx * 1.3;
        const px = Math.cos(a + t * 7 * side) * r, py = Math.sin(a + t * 7 * side) * r;
        if (a === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
      }
      ctx.stroke();
    } else if (kind === 'arch') {
      ctx.strokeStyle = ink; ctx.lineWidth = 3;
      ctx.beginPath(); ctx.arc(0, ry * 0.35, rx * 0.95 * pose.wide, Math.PI * 1.08, Math.PI * 1.92); ctx.stroke();
    } else if (kind === 'line') {
      ctx.strokeStyle = ink; ctx.lineWidth = 3;
      ctx.beginPath(); ctx.moveTo(-rx * 0.9, 0); ctx.quadraticCurveTo(0, ry * 0.18, rx * 0.9, 0); ctx.stroke();
    } else {
      ctx.scale((1 - 0.12 * Math.abs(look2.x)) * pose.wide, pose.open * (blinking ?? 1));
      ctx.fillStyle = ink;
      ctx.beginPath(); ctx.ellipse(0, 0, rx, ry, 0, 0, TAU); ctx.fill();
      ctx.fillStyle = 'rgba(90,165,215,.3)';
      ctx.beginPath(); ctx.ellipse(rx * 0.08, ry * 0.5, rx * 0.55, ry * 0.18, 0, 0, TAU); ctx.fill();
      ctx.fillStyle = 'rgba(255,255,255,.82)';
      ctx.beginPath(); ctx.arc(-rx * 0.3 + look2.x * rx * 0.12, -ry * 0.38 + look2.y * ry * 0.08, rx * 0.3, 0, TAU); ctx.fill();
      ctx.beginPath(); ctx.arc(rx * 0.32, ry * 0.32, rx * 0.16, 0, TAU); ctx.fill();
    }
    ctx.restore();
  }

  function drawMouth(kind, pose, x, y, w, h, t) {
    ctx.save();
    ctx.translate(x, y);
    ctx.scale(shown.mouthScale, shown.mouthScale);
    ctx.strokeStyle = ink; ctx.fillStyle = ink; ctx.lineWidth = 2.6; ctx.lineCap = 'round';
    const half = w * mouth.width * pose.mw;
    if (talk > 0.04) {
      const open = Math.min(1, talk * 1.6);
      ctx.beginPath(); ctx.ellipse(0, half * 0.05, half * (0.5 + 0.35 * open), half * (0.1 + 0.85 * open), 0, 0, TAU); ctx.fill();
      if (open > 0.45) { ctx.fillStyle = '#ff7a95'; ctx.beginPath(); ctx.ellipse(0, half * (0.1 + 0.55 * open), half * 0.38, half * 0.24 * open, 0, 0, TAU); ctx.fill(); }
    } else if (kind === 'o') {
      ctx.beginPath(); ctx.ellipse(0, 2, w * 0.024, h * 0.045, 0, 0, TAU); ctx.fill();
    } else if (kind === 'open') {
      ctx.beginPath(); ctx.ellipse(0, -half * 0.2, half * 0.95, half * 1.1, 0, 0, Math.PI); ctx.closePath(); ctx.fill();
      ctx.fillStyle = '#ff7a95';
      ctx.beginPath(); ctx.ellipse(0, half * 0.55, half * 0.5, half * 0.32, 0, 0, TAU); ctx.fill();
    } else if (kind === 'yawn') {
      const k = 0.75 + 0.25 * Math.sin(t * 3);
      ctx.beginPath(); ctx.ellipse(0, 0, half * 0.7 * k, half * 1.15 * k, 0, 0, TAU); ctx.fill();
      ctx.fillStyle = '#ff7a95';
      ctx.beginPath(); ctx.ellipse(0, half * 0.45 * k, half * 0.4 * k, half * 0.4 * k, 0, 0, TAU); ctx.fill();
    } else if (kind === 'wavy') {
      ctx.beginPath();
      for (let i = 0; i <= 12; i++) {
        const px = -half + (i / 12) * half * 2, py = Math.sin(i * 1.3 + t * 9) * half * 0.18;
        if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
      }
      ctx.stroke();
    } else {
      ctx.beginPath(); ctx.moveTo(-half, 0); ctx.quadraticCurveTo(0, pose.curve * half * 1.5, half, 0); ctx.stroke();
      if (kind === 'tongue') {
        ctx.fillStyle = '#ff7a95';
        ctx.beginPath(); ctx.ellipse(half * 0.35, half * 0.75, half * 0.34, half * 0.5, 0, 0, TAU); ctx.fill();
      }
    }
    ctx.restore();
  }

  function emit(state, fx, fy, m, t, dt) {
    nextFloater -= dt;
    if (nextFloater <= 0 && (state.hearts || state.zzz)) {
      floaters.push({ kind: state.hearts ? 'heart' : 'z', x: fx + (Math.random() - 0.3) * m.w * 0.3, y: fy - m.h * 0.2, age: 0, drift: (Math.random() - 0.5) * 8 });
      nextFloater = state.hearts ? 0.35 : 0.9;
    }
    for (let i = floaters.length - 1; i >= 0; i--) {
      const f = floaters[i];
      f.age += dt;
      f.y -= 22 * dt; f.x += f.drift * dt;
      if (f.age > 1.8) { floaters.splice(i, 1); continue; }
      const a = Math.min(1, f.age * 4) * (1 - f.age / 1.8);
      ctx.save();
      ctx.globalAlpha = a * 0.9;
      if (f.kind === 'heart') { ctx.fillStyle = '#ff6b92'; heart(ctx, f.x, f.y, 4 + f.age * 2); }
      else {
        ctx.strokeStyle = 'rgba(235,245,255,.95)'; ctx.lineWidth = 1.8; ctx.lineJoin = 'round';
        const s = 4 + f.age * 2.5;
        ctx.beginPath(); ctx.moveTo(f.x - s, f.y - s); ctx.lineTo(f.x + s, f.y - s); ctx.lineTo(f.x - s, f.y + s); ctx.lineTo(f.x + s, f.y + s); ctx.stroke();
      }
      ctx.restore();
    }
  }

  return {
    busy() { return moving; },
    /** The color of the eyes and mouth; null brings back the model's own (dark gel needs a light face). */
    setInk(color) { ink = color || baseInk; },
    /** Shows an expression for a few seconds (what the brain wants to say with its face). */
    hint(name, seconds) { hint = { name, until: now + seconds }; },
    /**
     * m: body measures in the body frame. input: { t, dt, look: {x, y} in -1..1 or null, vel, acc: body
     * velocity and acceleration in the body frame, sleepiness 0..1, grabbed, hover, speed, airborne }.
     */
    draw(m, input) {
      const { t, dt } = input;
      now = t;
      aim(input, t, dt);

      const trail = { x: -input.vel.x * lag.velocity, y: -input.vel.y * lag.velocity };
      const slosh = { x: -input.acc.x * lag.inertia, y: -input.acc.y * lag.inertia };
      const reachX = look.range[0] * m.w, reachY = look.range[1] * m.h;
      const aimed = trail;
      const limit = { x: 0.24 * m.w, y: 0.2 * m.h };
      integrate(center, { x: clamp(aimed.x, -limit.x, limit.x), y: clamp(aimed.y, -limit.y, limit.y) }, slosh, lag.stiffness, lag.damping, dt);
      integrate(left, { x: gaze.x * m.w * 0.03, y: gaze.y * m.h * 0.024 }, { x: slosh.x * 0.5, y: slosh.y * 0.5 }, lag.stiffness * 0.7, lag.damping * 0.8, dt);
      integrate(right, { x: gaze.x * m.w * 0.03, y: gaze.y * m.h * 0.024 }, { x: slosh.x * 0.62, y: slosh.y * 0.62 }, lag.stiffness * 0.62, lag.damping * 0.75, dt);
      integrate(lips, { x: gaze.x * m.w * 0.012, y: gaze.y * m.h * 0.01 }, { x: slosh.x * 0.8, y: slosh.y * 0.8 }, lag.stiffness * 0.5, lag.damping * 0.7, dt);
      for (const s of [center, left, right, lips]) {
        s.x = clamp(s.x, -0.3 * m.w, 0.3 * m.w);
        s.y = clamp(s.y, -0.26 * m.h, 0.26 * m.h);
      }

      talk += ((input.voice ?? 0) - talk) * Math.min(1, dt * 16);
      const state = machine.update({
        grabbed: !!input.grabbed, hover: !!input.hover, pointerSpeed: input.pointerSpeed ?? 0, speed: input.speed ?? 0, airborne: !!input.airborne,
        squash: m.squash, sleepiness: input.sleepiness ?? 0, acc: Math.hypot(input.acc.x, input.acc.y), sulking: !!input.sulking, hint,
        intent: input.intent, catchPhase: input.catchPhase, crouching: !!input.crouching, impact: input.impact ?? 0,
      }, t, dt);
      const pose = state.pose;
      pop(state.eye, state.eye, 'eye', dt);
      pop(state.mouth, state.mouth, 'mouth', dt);

      const restingWidth = config.body.radius * 2;
      const faceWidth = restingWidth * clamp(Math.sqrt(m.w / restingWidth), 0.85, 1.15);
      const rx = faceWidth * eyes.width, ry = rx * eyes.aspect;
      const sep = Math.min(faceWidth * eyes.separation, m.w * 0.2);
      const eyeDrift = m.w * 0.025;
      const faceReach = Math.max(0, m.w * 0.43 - sep - rx * 1.4 - eyeDrift);
      const fx = m.cx + clamp(gaze.x * reachX + center.x, -faceReach, faceReach), fy = m.minY + m.h * eyes.height + gaze.y * reachY + center.y;
      const inside = {
        x: (x) => clamp(x, m.cx - 0.4 * m.w, m.cx + 0.4 * m.w),
        y: (y) => clamp(y, m.minY + 0.2 * m.h, m.maxY - 0.15 * m.h),
      };
      const sleepiness = input.intent === 'wander' || input.intent === 'follow' ? 0 : input.sleepiness ?? 0;
      const lids = shown.eye === 'oval' ? blinkScale(t, sleepiness) * (1 - 0.58 * sleepiness) * Math.max(0.06, input.awake ?? 1) : 1;

      for (const [side, eye] of [[-1, left], [1, right]]) {
        const x = inside.x(fx + side * sep + clamp(eye.x, -eyeDrift, eyeDrift)), y = inside.y(fy + clamp(eye.y, -m.h * 0.04, m.h * 0.04));
        const near = 1 + 0.07 * side * gaze.x;
        drawEye(shown.eye, x, y, rx * near, ry * near, pose, side, t, gaze, lids);
      }
      drawMouth(shown.mouth, pose, inside.x(fx + lips.x), inside.y(fy + m.h * mouth.drop + lips.y), m.w, m.h, t);

      if (blush && pose.blush > 0.05) {
        ctx.fillStyle = `rgba(255,110,150,${0.38 * pose.blush})`;
        for (const side of [-1, 1]) {
          ctx.beginPath(); ctx.ellipse(fx + side * sep * 1.45 + center.x * 0.2, fy + m.h * 0.07, m.w * 0.035, m.h * 0.03, 0, 0, TAU); ctx.fill();
        }
      }
      emit(state, fx, fy, m, t, dt);
      const stirring = [center, left, right, lips].some((s) => Math.hypot(s.vx, s.vy) > 3);
      moving = stirring || blinkStart >= 0 || floaters.length > 0 || shown.eyeScale < 1 || shown.mouthScale < 1
        || state.name !== 'calm' || Math.hypot(gaze.x - (input.look?.x ?? wander.x), gaze.y - (input.look?.y ?? wander.y)) > 0.03;
    },
  };
}
