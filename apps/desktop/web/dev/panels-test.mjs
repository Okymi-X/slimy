// Memory and reminder panels, the fallback provider select, and a due reminder announced by the slime, on the real page
// (dev/app-harness.html, backend faked). Needs `python3 -m http.server 8765` in apps/desktop/web.
import { launch, sleep } from './cdp.mjs';

(async () => {
  const page = await launch({ provider: 'groq', port: '9471', tag: 'panels' });
  await sleep(5000);
  const checks = [];
  const check = (name, ok) => { checks.push(ok); console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}`); };
  const ev = (code) => page.evaluate(code);
  // The slime may ask one of its own questions first (slower machines): decline it so the bubble is free, then wait for the text.
  const bubbleHas = async (pattern) => {
    for (let i = 0; i < 40; i++) {
      await ev(`(() => { const b = document.querySelector('#bubble .ghost'); if (b && !b.hidden && b.offsetParent) b.click(); })()`);
      if (await ev(`${pattern}.test(document.querySelector('#bubble')?.textContent ?? '')`)) return true;
      await sleep(1500);
    }
    return false;
  };

  check('the morning briefing is said once at the first free moment', await bubbleHas('/Good morning|Bonjour/'));
  await ev(`window.__menu.find((i) => i.id === 'perms').action()`);
  await sleep(900);
  check('the memory panel lists what the slime keeps', await ev(`document.querySelector('#perms .memory-box')?.textContent.includes('favorite_color: green')`));
  check('the reminder panel lists the reminders to come', await ev(`document.querySelector('#perms .reminders-box')?.textContent.includes('stretch')`));
  await ev(`[...document.querySelectorAll('#perms .memory-box button')][0].click()`);
  await ev(`[...document.querySelectorAll('#perms .reminders-box button')][0].click()`);
  await sleep(400);
  check('forgetting a memory reaches the backend', await ev(`window.__calls.some((c) => c[0] === 'memory_forget' && c[1].key === 'favorite_color')`));
  check('removing a reminder reaches the backend', await ev(`window.__calls.some((c) => c[0] === 'reminder_remove' && c[1].id === 1)`));
  await page.shot('panels');

  await ev(`window.__menu.find((i) => i.id === 'brain').action()`);
  await sleep(900);
  const options = await ev(`[...document.querySelectorAll('#settings select')[1].options].map((o) => o.value)`);
  check('the fallback select offers none and each provider', options.join('|') === '|groq|openai');
  await ev(`(() => { const s = document.querySelectorAll('#settings select')[1]; s.value = 'openai'; s.dispatchEvent(new Event('change')); })()`);
  await ev(`[...document.querySelectorAll('#settings button')].find((b) => b.textContent === 'Save' || b.textContent === 'Enregistrer').click()`);
  await sleep(500);
  check('the chosen fallback is saved with the config', await ev(`window.__brainSaved?.fallback === 'openai'`));
  await ev(`(() => { const s = document.querySelectorAll('#settings select')[1]; s.value = 'groq'; s.dispatchEvent(new Event('change')); })()`);
  await ev(`[...document.querySelectorAll('#settings button')].find((b) => b.textContent === 'Save' || b.textContent === 'Enregistrer').click()`);
  await sleep(500);
  check('the main provider is never its own fallback', await ev(`window.__brainSaved?.fallback === null`));

  await ev(`document.querySelector('#settings .panel-buttons button:last-of-type')?.click()`);
  await ev(`window.__due = [{ text: 'drink water' }]`);
  check('a due reminder is announced in the bubble', await bubbleHas('/drink water/'));
  await page.shot('reminder');
  const errors = await page.errors();
  check('no page errors', errors.length === 0);
  if (errors.length) console.log(JSON.stringify(errors));
  page.close();
  console.log(checks.every(Boolean) ? 'RESULT PASS' : 'RESULT FAIL');
  process.exit(checks.every(Boolean) ? 0 : 1);
})();
