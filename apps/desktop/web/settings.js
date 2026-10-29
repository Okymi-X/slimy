// The brain settings: which provider, which model, which key. The key goes straight to the system
// keyring through the backend and is wiped from the field once saved; the page never reads it back.

import { createPlacer } from './placement.js';
import { explain, say } from './i18n.js';

function el(tag, props = {}, ...children) {
  const node = Object.assign(document.createElement(tag), props);
  node.append(...children);
  return node;
}

export function createSettings(root, { invoke, language, onClose, onSaved = () => {} }) {
  const placer = createPlacer(root);
  root.hidden = true;
  let providers = [];
  let loaded = null;
  let previousDefault = '';

  const provider = el('select');
  const note = el('p', { className: 'panel-note' });
  const model = el('input', { type: 'text', autocomplete: 'off', spellcheck: false, maxLength: 120 });
  const base = el('input', { type: 'text', autocomplete: 'off', spellcheck: false, maxLength: 200 });
  const region = el('input', { type: 'text', autocomplete: 'off', spellcheck: false, maxLength: 30 });
  const fallback = el('select');
  const key = el('input', { type: 'password', autocomplete: 'off', spellcheck: false, maxLength: 300 });
  const status = el('p', { className: 'panel-status', role: 'status' });
  const field = (label, input) => el('label', { className: 'panel-field' }, el('span', { textContent: label }), input);
  const baseRow = field(say(language, 'fieldBase'), base);
  const regionRow = field(say(language, 'fieldRegion'), region);
  const button = (name, handler, className = '') => el('button', { type: 'button', textContent: say(language, name), className, onclick: handler });

  root.append(
    el('h2', { textContent: say(language, 'settingsTitle') }),
    field(say(language, 'fieldProvider'), provider), note,
    field(say(language, 'fieldModel'), model), baseRow, regionRow,
    field(say(language, 'fieldFallback'), fallback), el('p', { className: 'panel-note', textContent: say(language, 'fallbackHint') }),
    field(say(language, 'fieldKey'), key), el('p', { className: 'panel-note', textContent: say(language, 'keyHint') }),
    el('div', { className: 'panel-buttons' }, button('save', () => save()), button('test', () => test(), 'ghost'), button('forget', () => forget(), 'ghost'), button('close', () => close(), 'ghost')),
    status,
  );

  const current = () => providers.find((p) => p.id === provider.value);

  async function refreshKeyState() {
    const stored = await invoke('brain_has_key', { provider: provider.value }).catch(() => false);
    key.placeholder = stored ? say(language, 'keyStored') : say(language, 'keyEmpty');
  }

  function applyProvider(keepModel) {
    const p = current();
    if (!p) return;
    if (!keepModel && (!model.value || model.value === previousDefault)) model.value = p.default_model;
    previousDefault = p.default_model;
    model.placeholder = p.default_model;
    base.placeholder = p.base_url || 'https://';
    regionRow.hidden = p.protocol !== 'bedrock';
    baseRow.hidden = p.protocol === 'bedrock';
    note.textContent = p.note;
    note.hidden = !p.note;
    refreshKeyState();
  }

  provider.addEventListener('change', () => applyProvider(false));

  async function save() {
    const p = current();
    const config = {
      provider: p.id,
      model: model.value.trim(),
      base_url: base.value.trim() || null,
      region: p.protocol === 'bedrock' ? region.value.trim() || null : null,
      temperature: loaded?.temperature ?? 0.8,
      max_tokens: loaded?.max_tokens ?? 220,
      fallback: fallback.value && fallback.value !== p.id ? fallback.value : null,
    };
    try {
      if (key.value.trim()) {
        await invoke('brain_set_key', { provider: p.id, key: key.value });
        key.value = '';
      }
      await invoke('brain_save', { config: JSON.stringify(config) });
      loaded = config;
      onSaved(config);
      status.textContent = say(language, 'saved');
      refreshKeyState();
      return true;
    } catch (error) {
      status.textContent = explain(language, error);
      return false;
    }
  }

  async function test() {
    if (!(await save())) return;
    status.textContent = say(language, 'testing');
    try {
      const reply = await invoke('brain_chat', { history: [{ role: 'user', text: 'Salut !' }], sulking: false });
      status.textContent = reply.say;
    } catch (error) {
      status.textContent = explain(language, error);
    }
  }

  async function forget() {
    try {
      await invoke('brain_forget_key', { provider: provider.value });
      status.textContent = say(language, 'keyForgotten');
      refreshKeyState();
    } catch (error) {
      status.textContent = explain(language, error);
    }
  }

  function close() {
    key.value = '';
    root.classList.remove('on');
    root.hidden = true;
    placer.reset();
    onClose();
  }

  root.addEventListener('keydown', (event) => { if (event.key === 'Escape') close(); });

  return {
    get visible() { return !root.hidden; },
    async open() {
      status.textContent = '';
      if (!providers.length) providers = await invoke('brain_providers');
      const { config } = await invoke('brain_config');
      loaded = config;
      provider.replaceChildren(...providers.map((p) => el('option', { value: p.id, textContent: p.name })));
      provider.value = config.provider;
      fallback.replaceChildren(el('option', { value: '', textContent: say(language, 'fallbackNone') }), ...providers.map((p) => el('option', { value: p.id, textContent: p.name })));
      fallback.value = config.fallback ?? '';
      model.value = config.model;
      base.value = config.base_url ?? '';
      region.value = config.region ?? '';
      applyProvider(true);
      root.hidden = false;
      placer.reset();
      requestAnimationFrame(() => { placer.measure(); root.classList.add('on'); provider.focus(); });
    },
    close,
    place: placer.place,
    rect: placer.rect,
  };
}
