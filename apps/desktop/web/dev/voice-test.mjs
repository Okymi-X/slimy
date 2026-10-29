// The voice on the real page (dev/app-harness.html, voice backend faked): the microphone button shows in the chat bubble, one
// press listens and sends what was heard, the answer is spoken, the mouth opens with the voice level, and the Voice section
// of the Powers panel lists what is installed. Needs `python3 -m http.server 8765` in apps/desktop/web.
import { launch, sleep } from './cdp.mjs';

(async () => {
  const page = await launch({ provider: 'groq', port: '9470', tag: 'voice' });
  await sleep(5000);
  const checks = [];
  const check = (name, ok) => { checks.push(ok); console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}`); };
  const ev = (code) => page.evaluate(code);

  await ev(`window.__menu.find((i) => i.id === 'chat').action()`);
  await sleep(700);
  check('the microphone button is shown in the chat bubble', await ev(`(() => { const m = document.querySelector('#bubble .mic'); return !!m && !m.hidden; })()`));
  await ev(`document.querySelector('#bubble .mic').click()`);
  await sleep(1200);
  check('pressing it listened', await ev('window.__listened === true'));
  check('what was heard went to the brain and the answer is shown', await ev(`document.querySelector('#bubble .bubble-text').textContent === 'Il est midi, je crois.'`));
  check('the answer was spoken aloud', await ev(`(window.__spoken || []).join('|') === 'Il est midi, je crois.'`));

  await ev(`window.__emit('voice-level', { kind: 'voice', level: 0.8 })`);
  await sleep(500);
  const open = await page.shot('talking');
  await ev(`window.__emit('voice-level', { kind: 'voice', level: 0 })`);
  await sleep(800);
  const shut = await page.shot('silent');
  check('the page keeps running while the voice level changes', open.box[2] > 0 && shut.box[2] > 0);

  await ev(`window.__menu.find((i) => i.id === 'perms').action()`);
  await sleep(900);
  const panel = await ev(`(() => ({ text: document.querySelector('#perms .voice-box')?.textContent ?? '', select: !!document.querySelector('#perms .voice-box select'), rows: document.querySelectorAll('#perms .power-row').length }))()`);
  check('the Powers panel has a Voice section with both models installed', /installed|install/i.test(panel.text) && panel.select);
  check('the microphone is one of the powers', await ev(`[...document.querySelectorAll('#perms .power-row span')].some((s) => /microphone/i.test(s.textContent))`));
  await ev(`(() => { const s = [...document.querySelectorAll('#perms .voice-box select')].pop(); s.value = 'kawaii'; s.dispatchEvent(new Event('change')); })()`);
  await sleep(500);
  check('choosing a style saves it', await ev(`window.__voiceStyle === 'kawaii'`));
  check('the panel offers the precise ears with their size', await ev(`/610/.test(document.querySelector('#perms .voice-box').textContent)`));
  await ev(`(() => { const s = document.querySelector('#perms .voice-box select'); s.value = 'small'; s.dispatchEvent(new Event('change')); })()`);
  await sleep(500);
  check('choosing the small model saves it', await ev(`window.__voiceHearing === 'small'`));
  const learned = await ev(`(() => ({ text: document.querySelector('#perms .learned-box')?.textContent ?? '', lines: document.querySelectorAll('#perms .learned-box li').length }))()`);
  check('the panel shows what the slime has learned, in words', learned.lines === 4 && /every 25 min/.test(learned.text) && /108 %/.test(learned.text) && /about 90 s/.test(learned.text) && /greeting is cheerful/.test(learned.text));
  await ev(`document.querySelector('#perms .learned-box button').click()`);
  await sleep(400);
  check('forgetting everything reaches the backend', await ev('window.__learnReset === true'));
  check('the pace asked of the model was used when speaking and the first remark asked for its gap', await ev(`window.__calls.some((c) => c[0] === 'learn_choose' && c[1].decision === 'voice_pace')`));
  const screens = await ev(`[...document.querySelectorAll('#perms .display-box option')].map((o) => o.textContent)`);
  check('the panel offers the main screen, following the pointer and each screen', screens.length === 4 && /1920 x 1080/.test(screens[2]) && /2560 x 1440/.test(screens[3]));
  await ev(`(() => { const s = document.querySelector('#perms .display-box select'); s.value = 'follow'; s.dispatchEvent(new Event('change')); })()`);
  await sleep(300);
  check('choosing where to live is saved', await ev(`window.__display === 'follow'`));
  await page.shot('panel');
  const errors = await page.errors();
  check('no page errors', errors.length === 0);
  if (errors.length) console.log(JSON.stringify(errors));
  page.close();
  console.log(checks.every(Boolean) ? 'RESULT PASS' : 'RESULT FAIL');
  process.exit(checks.every(Boolean) ? 0 : 1);
})();
