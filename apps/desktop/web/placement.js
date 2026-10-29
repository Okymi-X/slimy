// Keeps a floating card next to the slime: above it, or below when there is no room, inside the
// usable area. The card glides toward its place so it never jitters with the slime's wobble.

const MARGIN = 10;
const GAP = 18;

export function createPlacer(root) {
  const size = { w: 0, h: 0 };
  const glide = { x: 0, y: 0, ready: false };
  let placed = null;
  const measure = () => { size.w = root.offsetWidth; size.h = root.offsetHeight; };
  new ResizeObserver(measure).observe(root);

  return {
    measure,
    reset() { glide.ready = false; placed = null; },
    /** anchor: { x, top, bottom } of the slime; area and view in world px. */
    place(anchor, area, view, dt) {
      if (root.hidden || size.w === 0) return;
      const { w, h } = size;
      const left = Math.min(Math.max(anchor.x - w / 2, area.left + MARGIN), area.right - w - MARGIN);
      let top = anchor.top - h - GAP;
      const below = top < area.top + MARGIN;
      if (below) top = anchor.bottom + GAP;
      // Neither side has room (a tall card on a short screen): stay on screen, even over the slime.
      top = Math.min(Math.max(top, area.top + MARGIN), area.bottom - h - MARGIN);
      if (!glide.ready) { glide.x = left; glide.y = top; glide.ready = true; }
      else {
        const ease = 1 - Math.exp(-16 * dt);
        glide.x += (left - glide.x) * ease;
        glide.y += (top - glide.y) * ease;
      }
      root.classList.toggle('below', below);
      root.style.setProperty('--tail', `${Math.min(Math.max(anchor.x - glide.x, 18), w - 18)}px`);
      root.style.transform = `translate3d(${(glide.x - view.x).toFixed(1)}px, ${(glide.y - view.y).toFixed(1)}px, 0)`;
      placed = { x0: glide.x, y0: glide.y, x1: glide.x + w, y1: glide.y + h };
    },
    rect() { return root.hidden ? null : placed; },
  };
}
