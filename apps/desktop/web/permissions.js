// The permissions panel: one switch per power, a duration for the ones that watch or act on the user,
// the latest accesses, and a badge while any power is granted. The backend decides every access.

import { createPlacer } from './placement.js';
import { explain, say } from './i18n.js';
import { percent } from './voice.js';

const DURATIONS = [300, 900, 3600];

// The wording of each learned decision; unknown ones fall back to the pace line.
const LEARNED_TEXT = { remark_gap: 'learnedGap', voice_pace: 'learnedPace', walk_gap: 'learnedWalk', greeting_style: 'learnedGreet' };

/** The best value of a belief in words a person reads: minutes, a percentage, seconds or a greeting style. */
function learnedValue(language, belief) {
  if (belief.decision === 'remark_gap') return Math.round(belief.best / 60);
  if (belief.decision === 'walk_gap') return Math.round(belief.best);
  if (belief.decision === 'greeting_style') return say(language, `greetStyle_${Math.round(belief.best)}`);
  return Math.round(belief.best * 100);
}

function el(tag, props = {}, ...children) {
  const node = Object.assign(document.createElement(tag), props);
  node.append(...children);
  return node;
}

/** The rows of the panel for a backend state: which powers are on, and which need a duration. */
export function powerRows(state) {
  const active = new Set(state.active ?? []), sensitive = new Set(state.sensitive ?? []);
  return (state.powers ?? []).map((power) => ({ power, on: active.has(power), timed: sensitive.has(power) }));
}

/** True while at least one power is granted: the slime is then allowed to look, and says so. */
export const isWatching = (state) => (state?.active?.length ?? 0) > 0;

export function createPermissions(root, { invoke, language, voice = null, listen = () => {}, onClose = () => {}, onChange = () => {} }) {
  const placer = createPlacer(root);
  root.hidden = true;
  const list = el('div', { className: 'power-list' });
  const log = el('ul', { className: 'power-log', 'aria-label': say(language, 'permsLog') });
  const status = el('p', { className: 'panel-status', role: 'status' });
  const voiceBox = el('div', { className: 'voice-box' });
  const learnedBox = el('div', { className: 'learned-box' });
  const displayBox = el('div', { className: 'display-box' });
  const memoryBox = el('div', { className: 'memory-box' });
  const remindersBox = el('div', { className: 'reminders-box' });
  const progress = {};
  const close = () => { root.classList.remove('on'); root.hidden = true; placer.reset(); onClose(); };
  root.append(
    el('h2', { textContent: say(language, 'permsTitle') }),
    el('p', { className: 'panel-note', textContent: say(language, 'permsHint') }),
    list,
    el('h3', { textContent: say(language, 'voiceTitle') }), voiceBox,
    el('h3', { textContent: say(language, 'displayTitle') }), displayBox,
    el('h3', { textContent: say(language, 'memoryTitle') }), memoryBox,
    el('h3', { textContent: say(language, 'remindersTitle') }), remindersBox,
    el('h3', { textContent: say(language, 'learnedTitle') }), learnedBox,
    el('h3', { textContent: say(language, 'permsLog') }), log,
    el('div', { className: 'panel-buttons' },
      el('button', { type: 'button', className: 'ghost', textContent: say(language, 'permsRevokeAll'), onclick: () => change('perms_revoke', {}) }),
      el('button', { type: 'button', className: 'ghost', textContent: say(language, 'close'), onclick: close })),
    status,
  );

  async function change(command, args) {
    try { await invoke(command, args); status.textContent = ''; await refresh(); } catch (error) { status.textContent = explain(language, error); }
  }

  async function install(which) {
    progress[which] = 0;
    renderVoice();
    try { await invoke('voice_install', { which }); status.textContent = ''; } catch (error) { status.textContent = explain(language, error); }
    delete progress[which];
    await voice?.refresh();
    renderVoice();
  }

  async function saveVoice(replies, style, hearing = voice?.status?.config?.hearing ?? 'base') {
    try { await invoke('voice_config_save', { replies, style, hearing }); await voice?.refresh(); } catch (error) { status.textContent = explain(language, error); }
    renderVoice();
  }

  /** The voice section: what is installed, the voice this slime has, spoken replies and style. */
  function renderVoice() {
    const s = voice?.status;
    if (!s) { voiceBox.replaceChildren(el('p', { className: 'panel-note', textContent: say(language, 'voiceUnavailable') })); return; }
    const row = (which, label) => {
      const have = which === 'hearing' ? s.installed?.hearing_base : s.installed?.[which];
      const line = el('div', { className: 'power-row' }, el('span', { textContent: label }));
      if (have) line.append(el('span', { className: 'panel-note', textContent: say(language, 'voiceInstalled') }));
      else if (which in progress) line.append(el('span', { className: 'panel-note', textContent: `${progress[which]} %` }));
      else line.append(el('button', { type: 'button', textContent: say(language, 'voiceInstall', { size: s.megabytes?.[which] ?? '' }), onclick: () => install(which) }));
      return line;
    };
    const replies = el('input', { type: 'checkbox', checked: s.config?.replies !== false, onchange: () => saveVoice(replies.checked, style.value, model.value) });
    const style = el('select', { onchange: () => saveVoice(replies.checked, style.value, model.value) },
      ...['auto', 'kawaii', 'calm'].map((k) => el('option', { value: k, textContent: say(language, `voiceStyle_${k}`), selected: s.config?.style === k })));
    const model = el('select', { onchange: () => saveVoice(replies.checked, style.value, model.value) },
      ...['base', 'small'].map((k) => el('option', { value: k, textContent: say(language, `voiceModel_${k}`, { size: s.megabytes?.[k === 'base' ? 'hearing' : 'hearing_small'] ?? '' }) + (s.installed?.[`hearing_${k}`] ? '' : ` (${say(language, 'voiceNotInstalled')})`), selected: (s.config?.hearing ?? 'base') === k })));
    voiceBox.replaceChildren(
      row('hearing', say(language, 'voiceHearing')), row('hearing_small', say(language, 'voiceHearingSmall')),
      el('label', { className: 'power-row' }, el('span', { textContent: say(language, 'voiceModel') }), model),
      row('speech', say(language, 'voiceSpeech')),
      el('label', { className: 'power-row' }, el('span', { textContent: say(language, 'voiceReplies') }), replies),
      el('label', { className: 'power-row' }, el('span', { textContent: say(language, 'voiceStyle') }), style),
      s.voice ? el('p', { className: 'panel-note', textContent: say(language, 'voiceName', { name: s.voice.name }) }) : '',
    );
  }

  /** Which screen the slime lives on: the main one, the one where the pointer is, or a fixed one. */
  async function renderDisplay() {
    let info = null;
    try { info = await invoke('display_status'); } catch { /* an old backend */ }
    if (!info?.screens?.length) { displayBox.replaceChildren(); return; }
    const options = [['primary', say(language, 'displayPrimary')], ['follow', say(language, 'displayFollow')],
      ...info.screens.map((s, i) => [String(i), say(language, 'displayScreen', { n: i + 1, size: `${s.rect.w} x ${s.rect.h}` })])];
    const select = el('select', { onchange: async () => { try { await invoke('display_save', { monitor: select.value }); } catch (error) { status.textContent = explain(language, error); } } },
      ...options.map(([value, label]) => el('option', { value, textContent: label, selected: value === info.layout?.preference })));
    displayBox.replaceChildren(el('label', { className: 'power-row' }, el('span', { textContent: say(language, 'displayScreenLabel') }), select));
  }

  const rowWith = (text, label, action) => el('li', { className: 'power-row' }, el('span', { textContent: text }),
    el('button', { type: 'button', className: 'ghost', textContent: label, onclick: async () => { try { await action(); } catch (error) { status.textContent = explain(language, error); } } }));

  /** The facts the slime keeps about its human, each with a way to forget it. */
  async function renderMemory() {
    let facts = [];
    try { facts = await invoke('memory_list'); } catch { /* no soul yet */ }
    memoryBox.replaceChildren(facts.length
      ? el('ul', { className: 'power-log' }, ...facts.slice(0, 30).map((f) => rowWith(`${f.key}: ${f.value}`, say(language, 'memoryForget'), async () => { await invoke('memory_forget', { key: f.key }); renderMemory(); })))
      : el('p', { className: 'panel-note', textContent: say(language, 'memoryNothing') }));
  }

  /** The reminders still to come, each removable. */
  async function renderReminders() {
    let items = [];
    try { items = await invoke('reminder_list'); } catch { /* none */ }
    const when = (r) => new Date(r.at * 1000).toLocaleString(language, { dateStyle: 'short', timeStyle: 'short' });
    remindersBox.replaceChildren(items.length
      ? el('ul', { className: 'power-log' }, ...items.map((r) => rowWith(`${when(r)}${r.every_day ? ` (${say(language, 'reminderDaily')})` : ''}: ${r.text}`, say(language, 'remindersRemove'), async () => { await invoke('reminder_remove', { id: r.id }); renderReminders(); })))
      : el('p', { className: 'panel-note', textContent: say(language, 'remindersNothing') }));
  }

  /** What the slime has learned about its human's habits, in plain words, with a way to forget it all. */
  async function renderLearned() {
    let beliefs = [];
    try { beliefs = await invoke('learn_report'); } catch { /* the model may be absent */ }
    const parts = ['night', 'morning', 'afternoon', 'evening'];
    const lines = beliefs.map((b) => el('li', { textContent: say(language, LEARNED_TEXT[b.decision] ?? 'learnedPace', {
      when: say(language, `part_${parts[b.context]}`), value: learnedValue(language, b), tries: b.tries, sure: Math.round(b.confidence * 100) }) }));
    learnedBox.replaceChildren(
      lines.length ? el('ul', { className: 'power-log' }, ...lines) : el('p', { className: 'panel-note', textContent: say(language, 'learnedNothing') }),
      el('button', { type: 'button', className: 'ghost', textContent: say(language, 'learnedForget'), onclick: async () => { try { await invoke('learn_reset'); } catch (error) { status.textContent = explain(language, error); } renderLearned(); } }),
    );
  }

  listen('voice-install', (event) => {
    const { which, done, total } = event.payload ?? {};
    if (which && which in progress) { progress[which] = percent(done, total); renderVoice(); }
  });

  async function refresh() {
    const state = await invoke('perms_state');
    list.replaceChildren(...powerRows(state).map(({ power, on, timed }) => {
      const name = say(language, `power_${power}`);
      const row = el('div', { className: 'power-row' }, el('span', { textContent: name }));
      if (on) row.append(el('button', { type: 'button', className: 'ghost', textContent: say(language, 'permsRevoke'), onclick: () => change('perms_revoke', { power }) }));
      else if (!timed) row.append(el('button', { type: 'button', textContent: say(language, 'permsGrant'), onclick: () => change('perms_grant', { power, seconds: null }) }));
      else DURATIONS.forEach((seconds) => row.append(el('button', { type: 'button', textContent: `${seconds / 60} min`, onclick: () => change('perms_grant', { power, seconds }) })));
      return row;
    }));
    log.replaceChildren(...(state.journal ?? []).slice(0, 8).map((e) => el('li', { textContent: `${say(language, `power_${e.power}`)}: ${say(language, `verdict_${e.verdict}`)}${e.note ? ` (${e.note})` : ''}` })));
    await voice?.refresh();
    renderVoice();
    renderLearned();
    renderMemory();
    renderReminders();
    renderDisplay();
    onChange(state);
    return state;
  }

  root.addEventListener('keydown', (event) => { if (event.key === 'Escape') close(); });

  return {
    get visible() { return !root.hidden; },
    async open() {
      status.textContent = '';
      await refresh();
      root.hidden = false;
      placer.reset();
      requestAnimationFrame(() => { placer.measure(); root.classList.add('on'); });
    },
    refresh,
    close,
    place: placer.place,
    rect: placer.rect,
  };
}
