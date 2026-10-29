import test from 'node:test';
import assert from 'node:assert/strict';
import { bindPointer } from '../pointer.js';

function scene() {
  const canvas = new EventTarget(), windowTarget = new EventTarget(), documentTarget = new EventTarget();
  const captures = new Set();
  canvas.setPointerCapture = (id) => captures.add(id);
  canvas.hasPointerCapture = (id) => captures.has(id);
  canvas.releasePointerCapture = (id) => captures.delete(id);
  const pointer = { contact: 0, speed: 0, x: 0, y: 0 };
  bindPointer(canvas, { pointer, contact: { away: 0, hover: 1, down: 2 }, windowTarget, documentTarget,
    place: (event) => { pointer.x = event.clientX; pointer.y = event.clientY; } });
  const send = (type, fields = {}, target = canvas) => target.dispatchEvent(Object.assign(new Event(type), {
    pointerId: 1, isPrimary: true, button: 0, buttons: 1, clientX: 30, clientY: 40, ...fields,
  }));
  return { canvas, windowTarget, documentTarget, pointer, captures, send };
}

test('only the primary button starts a grab', () => {
  const s = scene();
  s.send('pointerdown', { button: 2, buttons: 2 });
  s.send('pointerdown', { button: 1, buttons: 4 });
  s.send('pointerdown', { isPrimary: false });
  assert.equal(s.pointer.contact, 0);
  assert.equal(s.captures.size, 0);
  s.send('pointerdown');
  assert.equal(s.pointer.contact, 2);
  assert.ok(s.captures.has(1));
});

for (const event of ['pointercancel', 'lostpointercapture', 'contextmenu', 'blur', 'visibilitychange']) {
  test(`${event} releases a grab without another pointerup`, () => {
    const s = scene();
    s.send('pointerdown');
    s.pointer.speed = 100;
    s.documentTarget.hidden = true;
    s.send(event, {}, event === 'blur' ? s.windowTarget : event === 'visibilitychange' ? s.documentTarget : s.canvas);
    assert.equal(s.pointer.contact, 0);
    assert.equal(s.pointer.speed, 0);
    assert.equal(s.captures.size, 0);
    s.send('pointerdown');
    assert.equal(s.pointer.contact, 2);
  });
}

test('a move with no held primary button repairs a missing release', () => {
  const s = scene();
  s.send('pointerdown');
  s.send('pointermove', { buttons: 0, clientX: 70 });
  assert.equal(s.pointer.contact, 1);
  assert.equal(s.pointer.x, 70);
  assert.equal(s.captures.size, 0);
});

test('normal dragging tracks the pointer and accepts release outside the canvas', () => {
  const s = scene();
  s.send('pointerdown');
  s.send('pointermove', { clientX: 80 });
  assert.equal(s.pointer.x, 80);
  assert.equal(s.pointer.contact, 2);
  s.send('pointerup', { buttons: 0, clientX: 90 }, s.windowTarget);
  assert.equal(s.pointer.x, 90);
  assert.equal(s.pointer.contact, 0);
  assert.equal(s.captures.size, 0);
});

test('unrelated pointers and secondary releases cannot interrupt a drag', () => {
  const s = scene();
  s.send('pointerdown');
  s.send('pointermove', { pointerId: 2, clientX: 90 });
  s.send('pointercancel', { pointerId: 2 });
  s.send('pointerup', { button: 2, buttons: 1 });
  assert.equal(s.pointer.x, 30);
  assert.equal(s.pointer.contact, 2);
});
