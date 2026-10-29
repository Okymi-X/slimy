// The speech bubble: a small glass card next to the slime, with an optional name field. Text is
// always inserted as text, never as markup, because it can contain what the user typed.

import { createPlacer } from './placement.js';

export function createDialogue(root, { onSubmit, onLater = () => {} }) {
  root.innerHTML = '';
  const text = document.createElement('p');
  text.className = 'bubble-text';
  const form = document.createElement('form');
  form.className = 'bubble-form';
  form.hidden = true;
  const input = document.createElement('input');
  input.type = 'text';
  input.maxLength = 24;
  input.autocomplete = 'off';
  input.spellcheck = false;
  const button = document.createElement('button');
  button.type = 'submit';
  const later = document.createElement('button');
  later.type = 'button';
  later.className = 'ghost';
  later.addEventListener('click', () => onLater());
  const mic = document.createElement('button');
  mic.type = 'button';
  mic.className = 'mic';
  mic.hidden = true;
  mic.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><rect x="9" y="3" width="6" height="11" rx="3" fill="currentColor"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>';
  let onMic = () => {};
  mic.addEventListener('click', () => onMic());
  form.append(input, mic, button, later);
  const error = document.createElement('p');
  error.className = 'bubble-error';
  error.setAttribute('role', 'alert');
  root.append(text, form, error);
  root.hidden = true;

  let busy = false;
  const placer = createPlacer(root);

  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    if (busy) return;
    busy = true;
    const problem = await onSubmit(input.value);
    busy = false;
    if (problem) error.textContent = problem;
    else { error.textContent = ''; input.value = ''; }
  });

  return {
    show({ message, field, problem }) {
      root.hidden = false;
      text.textContent = message;
      error.textContent = problem ?? '';
      form.hidden = !field;
      if (field) {
        input.placeholder = field.placeholder;
        input.setAttribute('aria-label', field.placeholder);
        button.textContent = field.submit;
        input.maxLength = field.maxLength ?? 24;
        input.hidden = !!field.confirm;
        mic.hidden = !field.mic || !!field.confirm;
        mic.classList.toggle('on', !!field.listening);
        mic.setAttribute('aria-label', field.micLabel ?? 'Microphone');
        mic.setAttribute('aria-pressed', field.listening ? 'true' : 'false');
        onMic = field.mic ?? (() => {});
        later.hidden = !field.later;
        later.textContent = field.later ?? '';
        if (field.confirm) button.focus({ preventScroll: true });
        else if (document.activeElement !== input) input.focus({ preventScroll: true });
      }
      placer.reset();
      requestAnimationFrame(() => { placer.measure(); root.classList.add('on'); });
    },
    hide() {
      root.classList.remove('on');
      root.hidden = true;
      placer.reset();
    },
    get visible() { return !root.hidden; },
    get text() { return text.textContent; },
    problem(message) { error.textContent = message; },
    place: placer.place,
    rect: placer.rect,
  };
}
