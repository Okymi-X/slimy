// Slime window: a transparent overlay covering the usable screen. The WASM simulation runs in
// screen coordinates at a fixed step, the renderer draws it, and only the slime's hit region
// receives the mouse.

import { createReminderPoller } from './reminders.js';
import { createBriefing } from './briefing.js';
import init, { Slime, Companion, Roamer, question_key as questionKey } from './pkg/slimy_wasm.js';
import { createRenderer, SPRITE } from './render.js';
import { loadModel } from './model.js';
import { createDialogue } from './dialogue.js';
import { createRitual } from './ritual.js';
import { createLife } from './life.js';
import { createRoam, createWalkUrge } from './roam.js';
import { profile as formProfile, formForProvider, formColors, formShape, formInset, formReach, formInk, insideFlat, bodyPointNear } from './forms.js';
import { createChat } from './chat.js';
import { createRemarks } from './remarks.js';
import { applyLook, createDayCounter, createThrowDetector, createNightJournal, localDay } from './genetics.js';
import { bindPointer } from './pointer.js';
import { createSettings } from './settings.js';
import { createPermissions, isWatching } from './permissions.js';
import { createVoice } from './voice.js';
import { createSlimeFile } from './slimefile.js';
import { createLearning } from './learn.js';
import { createRefreshClock, frameStride, strideThreshold } from './cadence.js';
import { detectLanguage, say } from './i18n.js';

const { invoke } = window.__TAURI__.core;
const appWindow = window.__TAURI__.window.getCurrentWindow();

const STEP = 1 / 120;
const MAX_CATCH_UP = 0.25;
const MIN_FRAME = 1 / 75;
const HIT_EVERY = 0.1;
const CONTACT = { away: 0, hover: 1, down: 2 };
const REST_ENTER_SPEED = 25;
const REST_LEAVE_SPEED = 60;
const REST_AFTER = 0.8;
const REST_FRAME = 1 / 12;
const REST_FRAME_BUSY = 1 / 30;
const REST_FRAME_DOZING = 1 / 6;
const POINTER_POLL_MS = 33;
const POINTER_POLL_REST_MS = 100;
const POINTER_POLL_DOZING_MS = 500;
const TETHER_STIFFNESS = 130;
const TETHER_DAMPING = 19;
const LEAVE_DELAY_MS = 160;
const ACTIVE_FOR = 1.5;
const GAZE_MEMORY = 6;
const POINTER_STALE = 20;
const DOZE_FROM = 45;
const DOZE_FULL = 110;

const canvas = document.getElementById('over');
const glCanvas = document.getElementById('gl');
const dpr = window.devicePixelRatio || 1;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));


/** Waits until the window really has the size of the play area, then says where its content starts on the desktop. */
async function settleView(area) {
  const wanted = (area.right - area.left) / dpr;
  for (let i = 0; i < 40 && Math.abs(innerWidth - wanted) > 2; i++) await sleep(50);
  await sleep(100);
  const inner = await appWindow.innerPosition();
  return { x: inner.x / dpr, y: inner.y / dpr };
}

async function coverScreen(area) {
  await invoke('cover_world', { area });
  return settleView(area);
}

async function start() {
  await init();
  const area = await invoke('world');
  const view = await coverScreen(area);

  const world = { left: area.left / dpr, top: area.top / dpr, right: area.right / dpr, bottom: area.bottom / dpr };
  const baseModel = await loadModel('./models/base-slime.json');
  // An existing slime is dressed by its genome; an egg gets the look of the hatchling it will become.
  let genomeState = null;
  let genomeSeed = null;
  let look = null;
  try {
    genomeState = await invoke('genome_state', { today: localDay() });
    look = genomeState?.look ?? null;
    if (!genomeState) {
      genomeSeed = Math.floor(Math.random() * 2 ** 52);
      look = await invoke('genome_preview', { seed: genomeSeed });
    }
  } catch (err) {
    report(`genome unavailable, base look used: ${err}`);
    genomeSeed = null;
  }
  const model = applyLook(baseModel, look);
  // A big or dense screen gets a bigger slime, a small one a smaller slime (the backend measures the screen).
  const sizeScale = Number.isFinite(area.size_scale) ? area.size_scale : 1;
  model.body.radius = Math.round(model.body.radius * sizeScale * 100) / 100;
  const radius = model.body.radius;
  const slime = new Slime(radius, world.left, world.right, world.bottom, world.top, (world.left + world.right) / 2, world.top + 120);
  const { shape, breathing } = model.body;
  slime.set_shape(shape.top, shape.bottom, shape.baseWidth);
  slime.set_breathing(breathing.speed, breathing.width, breathing.height);
  const renderer = createRenderer({ glCanvas, overCanvas: canvas, dpr, model });
  renderer.init();

  const pointer = { x: -1e6, y: -1e6, contact: CONTACT.away, speed: 0, movedAt: -1e9 };
  let lastActivity = performance.now() / 1000;
  const touched = () => { lastActivity = performance.now() / 1000; };
  const place = (e) => {
    const x = view.x + e.clientX, y = view.y + e.clientY, now = performance.now() / 1000;
    const moved = Math.hypot(x - pointer.x, y - pointer.y);
    if (moved > 0.5 && pointer.x > -1e5) {
      pointer.speed = 0.7 * pointer.speed + 0.3 * (moved / Math.max(now - pointer.movedAt, 1e-3));
      pointer.movedAt = now;
    }
    pointer.x = x; pointer.y = y;
    touched();
  };

  // The mouse outside the slime's hit region is invisible to the page, so the backend reports it.
  let wake = () => {}; // set with the frame loop: ends the wait of a resting slime at once
  let dozing = false; // asleep: the cursor is looked up less often, and a moving one wakes the slime at the next look
  const screenPointer = { x: 0, y: 0, known: false, since: -1e9 };
  let resting = false;
  const poll = async () => {
    try {
      const at = await invoke('pointer_position');
      if (at) {
        const x = at[0] / dpr, y = at[1] / dpr;
        if (!screenPointer.known || Math.hypot(x - screenPointer.x, y - screenPointer.y) > 1) {
          screenPointer.since = performance.now() / 1000;
          touched();
          wake();
        }
        Object.assign(screenPointer, { x, y, known: true });
      }
    } catch {
      screenPointer.known = false;
    }
    setTimeout(poll, dozing ? POINTER_POLL_DOZING_MS : resting && performance.now() / 1000 - screenPointer.since >= ACTIVE_FOR ? POINTER_POLL_REST_MS : POINTER_POLL_MS);
  };
  poll();
  bindPointer(canvas, { pointer, place, contact: CONTACT });
  let leaving = 0;
  canvas.addEventListener('pointerenter', () => clearTimeout(leaving));
  canvas.addEventListener('pointerleave', () => {
    clearTimeout(leaving);
    leaving = setTimeout(() => { if (pointer.contact !== CONTACT.down) pointer.contact = CONTACT.away; }, LEAVE_DELAY_MS);
  });

  const hit = { x: -1, y: -1, w: 0, h: 0, at: -1e9 };
  const syncHitRegion = (pts, extra, now, speed) => {
    let minX = 1e9, maxX = -1e9, minY = 1e9, maxY = -1e9;
    for (let i = 0; i < pts.length; i += 2) {
      minX = Math.min(minX, pts[i]); maxX = Math.max(maxX, pts[i]);
      minY = Math.min(minY, pts[i + 1]); maxY = Math.max(maxY, pts[i + 1]);
    }
    if (extra) {
      minX = Math.min(minX, extra.x0); maxX = Math.max(maxX, extra.x1);
      minY = Math.min(minY, extra.y0); maxY = Math.max(maxY, extra.y1);
    }
    const pad = 14 + Math.min(speed * 0.12, 80);
    const next = {
      x: Math.max(0, Math.floor((minX - view.x - pad) * dpr)),
      y: Math.max(0, Math.floor((minY - view.y - pad) * dpr)),
      w: Math.ceil((maxX - minX + 2 * pad) * dpr),
      h: Math.ceil((maxY - minY + 2 * pad) * dpr),
    };
    const moved = Math.max(Math.abs(next.x - hit.x), Math.abs(next.y - hit.y), Math.abs(next.w - hit.w), Math.abs(next.h - hit.h));
    // The shape change is a round trip to the window system: keep it rare so typing and drawing stay smooth.
    if (moved < 4 || now - hit.at < HIT_EVERY) return;
    Object.assign(hit, next, { at: now });
    invoke('set_hit_region', next);
  };

  const language = detectLanguage();
  const walkUrge = createWalkUrge({ learn: createLearning({ invoke }) });
  const roam = createRoam({ slime, roamer: new Roamer(Math.floor(Math.random() * 2 ** 31), world.left, world.right, world.top, world.bottom), radius, walk: walkUrge });
  let chat = null;
  let settings = null;
  let perms = null;
  let grantedPowers = new Set();
  const panelOpen = () => !!(settings?.visible || perms?.visible);
  let day = null, closing = false;
  const { Menu } = window.__TAURI__.menu;
  const menu = await Menu.new({ items: [
    { id: 'chat', text: say(language, 'chatMenu'), action: () => { if (ritualDone && !life?.asking && !panelOpen()) chat.start(); } },
    { id: 'brain', text: say(language, 'brainMenu'), action: () => { chat?.close(); perms.close(); settings.open(); } },
    { id: 'perms', text: say(language, 'permsMenu'), action: () => { chat?.close(); settings.close(); perms.open(); } },
    { id: 'slime-export', text: say(language, 'slimeExportMenu'), action: () => { if (!panelOpen() && !chat?.open && !life?.asking) slimeFile?.exportSoul(); } },
    { id: 'slime-import', text: say(language, 'slimeImportMenu'), action: () => { if (!panelOpen() && !chat?.open && !life?.asking) slimeFile?.startImport(); } },
    { id: 'roam', text: say(language, 'roamToggle'), action: () => { roam.enabled = !roam.enabled; } },
    { id: 'quit', text: say(language, 'quit'), action: async () => {
      if (closing) return;
      closing = true;
      if (day && !(await day.flush())) {
        closing = false;
        const message = say(language, 'saveBeforeQuit');
        if (dialogue.visible) dialogue.problem(message); else dialogue.show({ message });
        return;
      }
      await invoke('quit');
    } },
  ] });
  canvas.addEventListener('contextmenu', (e) => { e.preventDefault(); menu.popup(); });

  // The soul decides whether this is a birth or a return.
  let slimeFile = null;
  let answerHandler = async () => null;
  let laterHandler = () => {};
  let currentSulking = false;
  const dialogue = createDialogue(document.getElementById('bubble'), { onSubmit: (text) => (slimeFile?.pending ? slimeFile.yes() : answerHandler(text)), onLater: () => (slimeFile?.pending ? slimeFile.no() : laterHandler()) });
  slimeFile = createSlimeFile({ invoke, dialogue, language, onApplied: () => setTimeout(() => location.reload(), 1500) });
  let stored = null;
  try {
    stored = await invoke('soul_load');
  } catch (err) {
    report(`soul unreadable, birth skipped to protect the file: ${err}`);
    stored = 'unreadable';
  }
  const ritual = stored ? null : createRitual({ slime, renderer, dialogue, invoke, language, model, startAt: performance.now() / 1000, genomeSeed });
  if (ritual) answerHandler = (text) => ritual.answer(text);
  let ritualDone = !ritual;

  const journal = createNightJournal({ dialogue, language });
  day = createDayCounter({ invoke, onNight: (night) => journal.enqueue(night) });
  const detectThrow = createThrowDetector();
  const learning = createLearning({ invoke });
  const voice = createVoice({ invoke, language, learn: learning });
  setInterval(() => voice.tick(), 5000);
  voice.refresh().then(() => voice.warm());
  const reminders = createReminderPoller({
    invoke, say: (key, vars) => say(language, key, vars),
    busy: () => closing || !ritualDone || journal.active || !!life?.asking || chat.open || panelOpen() || dialogue.visible,
    announce: (message) => { dialogue.show({ message }); renderer.hint('happy', 3); wake(); },
  });
  setInterval(() => { reminders.poll().then(() => reminders.flush()); }, 20000);
  setInterval(() => reminders.flush(), 3000);
  const briefing = createBriefing({ invoke, say: (key, vars) => say(language, key, vars), busy: () => reminders.pending > 0 || closing || !ritualDone || journal.active || !!life?.asking || chat.open || panelOpen() || dialogue.visible, announce: (message) => { dialogue.show({ message }); renderer.hint('happy', 3); wake(); } });
  setInterval(() => { briefing(); }, 5000);
  // The loudness of the slime's own voice moves its mouth; the microphone's is not drawn.
  let voiceLevel = 0;
  window.__TAURI__.event?.listen('voice-level', (event) => { if (event.payload?.kind === 'voice') { voiceLevel = event.payload.level; wake(); } });
  chat = createChat({ dialogue, invoke, language, voice, onExpression: (name, seconds) => renderer.hint(name, seconds), isSulking: () => currentSulking, onMessage: () => { if (!closing) day.message(); remarks.interaction(performance.now() / 1000); walkUrge.interaction(performance.now() / 1000); } });
  // The body follows the brain: a new provider is a metamorphosis, and the slime's own colors come back with the plain form.
  let form = 'slime';
  let grabShift = null;
  let tether = null;
  // The physics keeps its own grips across a metamorphosis: a press held through one is ignored until it is released.
  let pressSuppressed = false;
  // Any new press or release ends the suppression, even between two frames.
  for (const type of ['pointerdown', 'pointerup', 'pointercancel']) canvas.addEventListener(type, () => { pressSuppressed = false; });
  const syncForm = (provider) => {
    const next = formForProvider(provider);
    if (next === form) return;
    form = next;
    // A hold on a limb of the old form would point at a vertex that no longer exists.
    grabShift = null; tether = null;
    pressSuppressed = pointer.contact === CONTACT.down;
    const [top, bottom, baseWidth] = formShape(next, [shape.top, shape.bottom, shape.baseWidth]);
    slime.set_shape(top, bottom, baseWidth);
    slime.set_profile(formProfile(next));
    roam.setInset(formInset(next));
    roam.setReach(formReach(next));
    renderer.setColors(formColors(next) ?? model.material.colors);
    renderer.setInk(formInk(next));
  };
  settings = createSettings(document.getElementById('settings'), { invoke, language, onClose: () => {}, onSaved: (config) => syncForm(config.provider) });
  const badge = document.getElementById('watching');
  badge.textContent = say(language, 'watching');
  perms = createPermissions(document.getElementById('perms'), { invoke, language, voice, listen: (name, handler) => window.__TAURI__.event?.listen(name, handler), onChange: (state) => { grantedPowers = new Set(state.active ?? []); badge.hidden = !isWatching(state); } });
  perms.refresh().catch(() => {});
  // A timed power ends by itself: look again now and then so the badge and the remarks follow.
  setInterval(() => perms.refresh().catch(() => {}), 30000);
  const loadForm = () => invoke('brain_config').then(({ config }) => syncForm(config.provider)).catch(() => {});
  if (!ritual) loadForm();

  // The application the user is looking at is read only while that power is granted; asking without it would only fill the journal with refusals.
  const remarks = createRemarks({ dialogue, invoke, language, learn: learning, onExpression: (name, seconds) => renderer.hint(name, seconds), context: () => (grantedPowers.has('active_app') ? invoke('active_app') : null) });

  // Daily life starts as soon as a soul exists: at once on a return, after the birth otherwise.
  let life = null;
  const startLife = (soulJson) => {
    const soul = JSON.parse(soulJson);
    const companion = new Companion(soul.relation.affection, soul.relation.trust, soul.relation.resentment);
    soul.facts.forEach((fact) => companion.know(fact.key));
    life = createLife({
      companion, dialogue, invoke, language, questionKey, startAt: performance.now() / 1000,
      onCaress: (seconds) => { if (!closing) day.caress(seconds); },
      onAnswered: () => { if (!closing) day.answered(); },
      onIgnored: () => { if (!closing) day.ignored(); },
      onCalmedAlone: () => { if (!closing) day.calmedAlone(); },
    });
    answerHandler = (text) => (chat.open ? chat.answer(text) : life.answer(text));
    laterHandler = () => (chat.open ? (chat.asking ? chat.decline() : chat.close()) : life.later());
  };
  if (stored && stored !== 'unreadable') startLife(stored);
  if (stored && stored !== 'unreadable') journal.enqueue(genomeState?.night);
  let sulkSide = 1;
  let wasSulking = false;
  canvas.addEventListener('pointerdown', (event) => {
    if (event.button === 0 && event.isPrimary !== false) ritual?.poke(performance.now() / 1000);
  });

  const lastSeen = { x: 0, y: 0, at: -1e9 };
  const gazePointer = (now) => {
    if (pointer.contact !== CONTACT.away) {
      Object.assign(lastSeen, { x: pointer.x, y: pointer.y, at: now });
      return { x: pointer.x, y: pointer.y, hover: true, known: true, speed: pointer.speed, active: now - pointer.movedAt < ACTIVE_FOR };
    }
    if (screenPointer.known && now - screenPointer.since < POINTER_STALE) {
      Object.assign(lastSeen, { x: screenPointer.x, y: screenPointer.y, at: now });
      return { x: screenPointer.x, y: screenPointer.y, hover: false, known: true, speed: 0, active: now - screenPointer.since < ACTIVE_FOR };
    }
    // Keep looking where the cursor was last seen before drifting back to idle.
    return { x: lastSeen.x, y: lastSeen.y, hover: false, known: now - lastSeen.at < GAZE_MEMORY, speed: 0, active: false };
  };

  let roaming = { gaze: null, intent: 'idle', crouching: false };
  let before = null;
  let appFocused = false;
  let last = performance.now() / 1000;
  let owed = 0;
  let calm = 0;
  // A resting slime draws a few times a second at most. Asking for an animation frame at every display refresh just to
  // skip it wakes the web process and the window system sixty times a second for nothing: wait on a timer instead.
  let timer = 0;
  const later = (seconds) => {
    clearTimeout(timer);
    timer = setTimeout(() => { timer = 0; requestAnimationFrame(frame); }, Math.max(1, seconds * 1000));
  };
  wake = () => { if (timer) { clearTimeout(timer); timer = 0; requestAnimationFrame(frame); } };
  for (const type of ['pointerenter', 'pointermove', 'pointerdown', 'pointerup']) canvas.addEventListener(type, () => wake());

  // The setup of the screens can change while the app runs (a screen plugged in, a new resolution, another screen chosen):
  // the backend moves the window and says so; the slime keeps living inside the new play area.
  window.__TAURI__.event?.listen('world-changed', async (event) => {
    const next = event.payload;
    if (!next || !Number.isFinite(next.right)) return;
    // Another pixel density or a very different screen size would need a new body: start over (the soul is kept by the backend).
    if ((window.devicePixelRatio || 1) !== dpr || Math.abs((next.size_scale ?? 1) - sizeScale) >= 0.15) { location.reload(); return; }
    const moved = await settleView(next);
    view.x = moved.x; view.y = moved.y;
    world.left = next.left / dpr; world.top = next.top / dpr; world.right = next.right / dpr; world.bottom = next.bottom / dpr;
    slime.resize_world(world.left, world.right, world.bottom, world.top);
    roam.setWorld(world);
    wake();
  });
  // Walking alone, nobody touching it: about 40 images a second read as smooth and cost far less than the display rate.
  const refreshClock = createRefreshClock();
  const frame = (ms) => {
    const now = ms / 1000;
    resting = calm > REST_AFTER && now - lastActivity > ACTIVE_FOR && !roaming.crouching && !['wander', 'follow', 'catch'].includes(roaming.intent) && pointer.contact === CONTACT.away && ritualDone && !journal.active && !life?.asking && !chat.open && !panelOpen();
    const sleepiness = Math.min(1, Math.max(0, (now - lastActivity - DOZE_FROM) / (DOZE_FULL - DOZE_FROM)));
    const wait = renderer.busy() ? REST_FRAME_BUSY : sleepiness > 0.5 ? REST_FRAME_DOZING : REST_FRAME;
    dozing = resting && sleepiness > 0.5;
    if (resting && now - last < wait) {
      later(wait - (now - last));
      return;
    }
    const refresh = refreshClock.tick(now);
    const cruising = roaming.intent === 'wander' && pointer.contact === CONTACT.away && !roaming.crouching;
    if (!resting && now - last < (cruising ? strideThreshold(refresh, frameStride(refresh)) : MIN_FRAME)) {
      requestAnimationFrame(frame);
      return;
    }
    const dt = Math.min(now - last, MAX_CATCH_UP);
    last = now;

    // A press on a limb (a tail, a claw) holds the limb: the gel under the pointer follows it and stretches, and
    // the body is dragged by the limb's root on a soft spring, so it trails behind like the end of an elastic rope.
    let aim = pointer;
    if (pointer.contact !== CONTACT.down) pressSuppressed = false;
    const contact = pressSuppressed && pointer.contact === CONTACT.down ? CONTACT.hover : pointer.contact;
    if (contact !== CONTACT.down) {
      grabShift = null;
      tether = null;
    } else if (grabShift) {
      const goal = { x: pointer.x + grabShift.x, y: pointer.y + grabShift.y };
      const total = Math.min(dt, MAX_CATCH_UP), n = Math.max(1, Math.ceil(total / (1 / 90))), h = total / n;
      for (let i = 0; i < n; i++) {
        tether.vx += ((goal.x - tether.x) * TETHER_STIFFNESS - tether.vx * TETHER_DAMPING) * h;
        tether.vy += ((goal.y - tether.y) * TETHER_STIFFNESS - tether.vy * TETHER_DAMPING) * h;
        tether.x += tether.vx * h; tether.y += tether.vy * h;
      }
      aim = { x: tether.x, y: tether.y };
    } else if (!slime.is_grabbed()) {
      const outline = slime.points();
      if (!insideFlat(outline, pointer.x, pointer.y) && renderer.limbContains(pointer.x, pointer.y)) {
        const near = bodyPointNear(outline, pointer.x, pointer.y);
        grabShift = { x: near.x - pointer.x, y: near.y - pointer.y };
        tether = { x: near.x, y: near.y, vx: 0, vy: 0 };
        aim = near;
      }
    }

    let impact = 0;
    owed += dt;
    const gaze = gazePointer(now);
    while (owed >= STEP) {
      if (owed < 2 * STEP) before = slime.points();
      roaming = roam.update(now - owed, {
        busy: closing || !ritualDone || journal.active || !!life?.asking || chat.open || panelOpen(),
        sulking: currentSulking, sleepiness, affection: life?.affection ?? 0.5, pointer: gaze,
      }, STEP);
      slime.step(STEP, aim.x, aim.y, contact);
      impact = Math.max(impact, slime.last_impact());
      owed -= STEP;
    }
    const speed = slime.speed();
    if (life && !closing) {
      if (detectThrow(slime.is_grabbed(), speed)) { day.thrown(); walkUrge.shoo(); }
      day.tick(now, dt, now - lastActivity);
    }
    const settling = speed < (resting ? REST_LEAVE_SPEED : REST_ENTER_SPEED) && pointer.contact === CONTACT.away;
    calm = settling ? calm + dt : 0;
    pointer.speed *= Math.exp(-5 * dt);

    const latest = slime.points();
    const blend = before ? owed / STEP : 1;
    const pts = before ? latest.map((v, i) => before[i] + (v - before[i]) * blend) : latest;
    const extras = ritual && !ritualDone ? ritual.update(now) : null;
    if (ritual && !ritualDone && ritual.finished) {
      ritualDone = true;
      loadForm();
      invoke('soul_load').then((json) => json && startLife(json)).catch(report);
    }
    const stroking = pointer.contact === CONTACT.hover && pointer.speed > 40 && now - pointer.movedAt < ACTIVE_FOR;
    const mood = life ? life.update(now, { busy: journal.active || chat.open || panelOpen() || closing, stroking, active: now - lastActivity < 0.3, idleFor: now - lastActivity }) : { sulking: false };
    if (life) {
      const { returned, caressed } = life.moment;
      // Touching the slime or writing to it right after a remark is how it learns the remark was welcome.
      if (stroking || pointer.contact === CONTACT.down) { remarks.interaction(now); walkUrge.interaction(now); }
      remarks.update(now, { returned, caressed, sulking: mood.sulking, free: !life.asking && !journal.active && !chat.open && !panelOpen() && !closing && ritualDone });
    }
    if (mood.sulking && !wasSulking) sulkSide = Math.random() < 0.5 ? -1 : 1;
    wasSulking = mood.sulking;
    currentSulking = mood.sulking;
    renderer.setDull(mood.sulking ? 1 : 0);
    journal.tick(now, !life || !!life.asking || chat.open || panelOpen() || closing);
    if (extras?.asking && !appFocused) { appFocused = true; appWindow.setFocus(); }
    let top = 1e9, bottom = -1e9, sumX = 0, sumY = 0;
    for (let i = 0; i < pts.length; i += 2) { top = Math.min(top, pts[i + 1]); bottom = Math.max(bottom, pts[i + 1]); sumX += pts[i]; sumY += pts[i + 1]; }
    const anchor = { x: sumX / (pts.length / 2), top, bottom };
    dialogue.place(anchor, world, view, dt);
    settings.place(anchor, world, view, dt);
    perms.place(anchor, world, view, dt);
    const cards = [dialogue.rect(), settings.rect(), perms.rect()].filter(Boolean);
    const cardBox = cards.length ? { x0: Math.min(...cards.map((c) => c.x0)), y0: Math.min(...cards.map((c) => c.y0)), x1: Math.max(...cards.map((c) => c.x1)), y1: Math.max(...cards.map((c) => c.y1)) } : null;
    const limbBox = renderer.limbBox();
    const touchBox = !limbBox ? cardBox : !cardBox ? limbBox : { x0: Math.min(cardBox.x0, limbBox.x0), y0: Math.min(cardBox.y0, limbBox.y0), x1: Math.max(cardBox.x1, limbBox.x1), y1: Math.max(cardBox.y1, limbBox.y1) };
    syncHitRegion(pts, touchBox, now, speed);
    const sprite = { x: Math.round(sumX / (pts.length / 2) - SPRITE / 2), y: Math.round(sumY / (pts.length / 2) - SPRITE / 2) };
    const shift = `translate(${sprite.x - view.x}px, ${sprite.y - view.y}px)`;
    canvas.style.transform = shift;
    glCanvas.style.transform = shift;
    renderer.draw({
      view: sprite,
      pts,
      restHeight: slime.rest_height(),
      radius,
      floorY: world.bottom,
      t: now,
      dt,
      pointer: gaze,
      egg: extras?.egg ?? null,
      awake: extras ? extras.awake : 1,
      sulking: mood.sulking,
      lookOverride: extras?.asking || life?.asking || chat.open ? { x: 0, y: -0.7 } : (roaming.intent === 'follow' && gaze.known ? null : roaming.gaze) ?? (mood.sulking ? { x: 0.9 * sulkSide, y: 0.25 } : null),
      sleepiness,
      form, limbGrab: !!grabShift, intent: roaming.intent, catchPhase: roaming.phase, crouching: roaming.crouching, impact, voice: voiceLevel,
      grabbed: slime.is_grabbed(),
      orientation: slime.orientation(),
      gap: slime.surface_gap(),
      gravityUp: (([x, y]) => ({ x, y }))(slime.gravity_up()),
    });
    if (resting) later(wait); else requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

const report = (err) => invoke('log_error', { message: String(err?.stack ?? err) });
addEventListener('error', (e) => report(e.error ?? e.message));
addEventListener('unhandledrejection', (e) => report(e.reason));
start().catch(report);
