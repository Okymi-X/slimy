// The morning briefing: once per local day, at the first moment the slime is free, it says what is coming today and how much it
// remembers. Built from the reminder and memory lists only, no model call, so it costs nothing and cannot invent anything.

const KEY = 'slimy.briefed';
const dayKey = (date) => `${date.getFullYear()}-${date.getMonth() + 1}-${date.getDate()}`;

/** The reminders that fall on `date`'s local day (daily ones always), earliest first, as `{ time, text }`. */
export function todaysReminders(list, date = new Date()) {
  const start = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime() / 1000;
  const end = start + 86400;
  const clock = (epoch) => { const d = new Date(epoch * 1000); return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`; };
  return (Array.isArray(list) ? list : [])
    .filter((r) => r && typeof r.text === 'string' && (r.every_day || (r.at >= start && r.at < end)))
    .sort((a, b) => a.at - b.at)
    .map((r) => ({ time: clock(r.at), text: r.text.replace(/\s+/g, ' ').trim().slice(0, 60) }));
}

/** The sentence to say, or null when there is nothing worth saying. */
export function briefingMessage({ reminders, facts, say }) {
  const parts = [];
  if (reminders.length) parts.push(say('briefingReminders', { list: reminders.slice(0, 3).map((r) => `${r.time} ${r.text}`).join(', '), more: reminders.length > 3 ? say('briefingMore', { n: reminders.length - 3 }) : '' }));
  if (facts > 0) parts.push(say('briefingFacts', { n: facts }));
  return parts.length ? `${say('briefingHello')} ${parts.join(' ')}` : null;
}

export function createBriefing({ invoke, say, busy, announce, storage = globalThis.localStorage, now = () => new Date() }) {
  let done = false;
  const seen = () => { try { return storage?.getItem(KEY); } catch { return null; } };
  const mark = (key) => { try { storage?.setItem(KEY, key); } catch { /* the briefing may repeat after a restart */ } };
  return async function tick() {
    if (done || busy()) return;
    const date = now();
    const key = dayKey(date);
    if (seen() === key) { done = true; return; }
    let reminders = [], facts = 0;
    try { reminders = todaysReminders(await invoke('reminder_list'), date); } catch { /* none */ }
    try { facts = (await invoke('memory_list')).length; } catch { /* no soul yet */ }
    done = true;
    mark(key);
    const message = briefingMessage({ reminders, facts, say });
    if (message) announce(message);
  };
}
