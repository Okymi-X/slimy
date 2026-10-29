// Export and import of the slime's soul as a `.slime` file. The backend owns the folder and every
// check; this only asks, shows the Yes/No question and reports. A file is never applied without a Yes.

import { say, explain } from './i18n.js';

/** A file name the backend would accept: a bare name ending in .slime, nothing that looks like a path. */
export function isPlainSlimeName(name) {
  return typeof name === 'string' && name.length > 6 && name.length <= 80 && /^[A-Za-z0-9][A-Za-z0-9._-]*\.slime$/.test(name) && !name.includes('..');
}

/** What the preview from the backend must look like before the page puts a question on screen. */
export function readPreview(value) {
  if (!value || !isPlainSlimeName(value.file) || typeof value.slime_name !== 'string') return null;
  const count = (n) => (Number.isInteger(n) && n >= 0 ? n : null);
  const nights = count(value.nights);
  const facts = count(value.facts);
  if (nights === null || facts === null) return null;
  return { file: value.file, name: value.slime_name.slice(0, 24), nights, facts };
}

export function createSlimeFile({ invoke, dialogue, language, onApplied }) {
  let pending = null;

  const confirmField = () => ({ confirm: true, placeholder: say(language, 'slimeImportAsk'), submit: say(language, 'yes'), later: say(language, 'no') });
  const tell = (message) => dialogue.show({ message });
  const fail = (err) => tell(explain(language, err));

  return {
    get pending() { return pending !== null; },

    async exportSoul() {
      try {
        const path = await invoke('slime_export');
        tell(say(language, 'slimeExported', { path }));
      } catch (err) { fail(err); }
    },

    async startImport() {
      try {
        const preview = readPreview(await invoke('slime_import_preview'));
        if (!preview) { pending = null; tell(say(language, 'slime_malformed')); return; }
        pending = preview;
        dialogue.show({ message: say(language, 'slimeImportConfirm', { name: preview.name, nights: preview.nights, facts: preview.facts, file: preview.file }), field: confirmField() });
      } catch (err) { pending = null; fail(err); }
    },

    /** The Yes of the bubble. Returns a problem text to show, or null. */
    async yes() {
      const choice = pending;
      pending = null;
      if (!choice) return null;
      try {
        await invoke('slime_import_apply', { file: choice.file });
        tell(say(language, 'slimeImported'));
        onApplied?.();
      } catch (err) { fail(err); }
      return null;
    },

    /** The No of the bubble: nothing was changed. */
    no() {
      if (!pending) return;
      pending = null;
      tell(say(language, 'slimeImportCancelled'));
    },
  };
}
