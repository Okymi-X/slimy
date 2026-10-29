// A grab belongs to one primary pointer and ends even when a native menu takes focus.
export function bindPointer(canvas, { pointer, place, contact, windowTarget = window, documentTarget = document }) {
  let held = null;
  const release = () => {
    const id = held;
    held = null;
    pointer.contact = contact.away;
    pointer.speed = 0;
    if (id !== null && canvas.hasPointerCapture(id)) canvas.releasePointerCapture(id);
  };
  canvas.addEventListener('pointerdown', (event) => {
    if (event.button !== 0 || event.isPrimary === false || held !== null) return;
    place(event);
    held = event.pointerId;
    pointer.contact = contact.down;
    canvas.setPointerCapture(held);
  });
  canvas.addEventListener('pointermove', (event) => {
    if (event.isPrimary === false || (held !== null && event.pointerId !== held)) return;
    if (held !== null && !(event.buttons & 1)) release();
    place(event);
    if (held === null) pointer.contact = contact.hover;
  });
  const up = (event) => {
    if (event.pointerId !== held) return;
    // Releasing a secondary mouse button must not release the primary drag.
    if (event.buttons & 1) return;
    place(event);
    release();
  };
  canvas.addEventListener('pointerup', up);
  windowTarget.addEventListener('pointerup', up);
  canvas.addEventListener('pointercancel', (event) => { if (event.pointerId === held) release(); });
  canvas.addEventListener('lostpointercapture', (event) => { if (event.pointerId === held) release(); });
  canvas.addEventListener('contextmenu', release);
  windowTarget.addEventListener('blur', release);
  documentTarget.addEventListener('visibilitychange', () => { if (documentTarget.hidden) release(); });
}
