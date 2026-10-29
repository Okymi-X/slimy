// Frame pacing for a free walk: draw every n-th display frame, so the images stay evenly spaced, with n chosen
// to land closest to the target rate on the user's own screen (a fixed time threshold gave 30 images a second on
// 60 Hz and 90 Hz screens and 40 only on 120 Hz).

export const TARGET_FPS = 40;

/** How many display frames to let pass between two drawn images for a screen refreshing every `refresh` seconds. */
export function frameStride(refresh, target = TARGET_FPS) {
  if (!(refresh > 0.001 && refresh < 0.1)) return 1;
  const hz = 1 / refresh;
  let best = 1;
  for (let n = 2; n <= Math.ceil(hz); n++) if (Math.abs(hz / n - target) < Math.abs(hz / best - target)) best = n;
  return Math.abs(hz - target) <= Math.abs(hz / best - target) ? 1 : best;
}

/** Minimum time since the last drawn image before drawing again: halfway between two display frames of slack. */
export const strideThreshold = (refresh, stride) => (stride - 0.5) * refresh;

/** Tracks the display refresh interval from consecutive animation frame timestamps (seconds). */
export function createRefreshClock() {
  let previous = 0, interval = 1 / 60;
  return {
    get interval() { return interval; },
    tick(now) {
      const delta = now - previous;
      previous = now;
      if (delta > 0.002 && delta < 0.05) interval += (Math.min(delta, interval * 1.5) - interval) * 0.05;
      return interval;
    },
  };
}
