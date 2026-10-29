// Reminders and what the slime shows before it acts: turning "in 20 minutes" or "at 18:30" into a real time, and describing a
// tool request in a few words so the person sees exactly what they are about to allow.

export const MAX_MINUTES = 7 * 24 * 60;

/** `{ minutes }` or `{ at: "HH:MM" }` as Unix seconds (the next such time, local), with a short label; null when unusable. */
export function whenToEpoch(args, now = new Date()) {
  const base = Math.floor(now.getTime() / 1000);
  const minutes = Number(args?.minutes);
  if (Number.isFinite(minutes) && minutes >= 1 && minutes <= MAX_MINUTES) return { at: base + Math.round(minutes) * 60, label: `+${Math.round(minutes)} min` };
  const match = /^\s*(\d{1,2})\s*[:h]\s*(\d{2})?\s*$/i.exec(String(args?.at ?? ''));
  if (!match) return null;
  const hour = Number(match[1]), minute = Number(match[2] ?? 0);
  if (hour > 23 || minute > 59) return null;
  const next = new Date(now);
  next.setHours(hour, minute, 0, 0);
  if (next.getTime() <= now.getTime()) next.setDate(next.getDate() + 1);
  return { at: Math.floor(next.getTime() / 1000), label: `${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}` };
}

const short = (value, max = 60) => String(value ?? '').replace(/\s+/g, ' ').trim().slice(0, max);

/** A few words on what a tool request will do, for the yes or no. */
export function describeRequest(tool) {
  const a = tool?.arguments ?? {};
  if (tool?.server === 'slimy') {
    if (tool.name === 'files') return a.path ? `"${short(a.path)}"` : '';
    if (tool.name === 'remember') return `${short(a.key, 30)} = ${short(a.value)}`;
    if (tool.name === 'remind') { const w = whenToEpoch(a); return `"${short(a.text)}"${w ? ` (${w.label}${a.every_day ? ', daily' : ''})` : ''}`; }
    if (tool.name === 'pointer') return `${Math.round(Number(a.dx) || 0)}, ${Math.round(Number(a.dy) || 0)} px`;
    if (tool.name === 'skill') return `"${short(a.name, 40)}"`;
    return '';
  }
  const text = Object.keys(a).length ? JSON.stringify(a) : '';
  return short(text, 80);
}

/** Polls the backend for due reminders and announces them one at a time once the slime is free. */
export function createReminderPoller({ invoke, say, busy, announce }) {
  const queue = [];
  const poll = async () => {
    try {
      const due = await invoke('reminders_due');
      if (Array.isArray(due)) due.forEach((r) => queue.push(String(r.text ?? '')));
    } catch { /* the store may be unreadable; the next poll tries again */ }
  };
  const flush = () => {
    if (!queue.length || busy()) return;
    announce(say('reminderNow', { text: queue.shift() }));
  };
  return { poll, flush, get pending() { return queue.length; } };
}
