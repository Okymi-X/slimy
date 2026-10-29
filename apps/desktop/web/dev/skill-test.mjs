// Two chat flows on the real page (dev/app-harness.html, backend faked): the slime asks for the built-in skill tool
// (Yes reads it and the text comes back as a tool result, No calls nothing), and a rate-limited brain gives a short readable
// message. Needs `python3 -m http.server $HARNESS_PORT` in apps/desktop/web.
import { launch, sleep } from './cdp.mjs';

(async () => {
  const page = await launch({ provider: 'groq', port: process.env.CHROME_PORT ?? '9472', tag: 'skill' });
  await sleep(5000);
  const checks = [];
  const check = (name, ok) => { checks.push(ok); console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}`); };
  const ev = (code) => page.evaluate(code);
  const text = () => ev(`document.querySelector('#bubble .bubble-text').textContent`);
  const send = async (value) => {
    await ev(`(() => { const i = document.querySelector('#bubble input'); i.value = ${JSON.stringify(value)}; i.closest('form').requestSubmit(); })()`);
    await sleep(900);
  };
  // 'yes' is the submit button, 'no' the ghost one.
  const press = async (which) => {
    await ev(`document.querySelector('#bubble .bubble-form ${which === 'yes' ? 'button[type=submit]' : 'button.ghost'}').click()`);
    await sleep(900);
  };
  const skillCalls = () => ev(`window.__calls.filter((c) => c[0] === 'skill_read').length`);
  const ask = { say: 'Let me check my notes.', expression: 'happy', tool: { server: 'slimy', name: 'skill', arguments: { name: 'standup' } } };

  await ev(`window.__menu.find((i) => i.id === 'chat').action()`);
  await sleep(700);

  await ev(`window.__brain = [${JSON.stringify(ask)}, { say: 'Standup: done, next, blocked.' }]`);
  await send('help me with my standup');
  const question = await text();
  check('the question names the skill tool and the skill', /slimy/.test(question) && /skill/.test(question) && /standup/.test(question));
  check('the Yes and No buttons are shown', await ev(`(() => { const b = [...document.querySelectorAll('#bubble .bubble-form button')].filter((x) => !x.hidden).map((x) => x.textContent); return b.includes('Yes') && b.includes('No'); })()`));
  check('nothing was read before the answer', (await skillCalls()) === 0);
  await press('yes');
  check('the skill was read by name', await ev(`window.__calls.some((c) => c[0] === 'skill_read' && c[1].name === 'standup')`));
  check('the final answer is shown', (await text()) === 'Standup: done, next, blocked.');
  const sent = await ev(`window.__brainSeen[1].map((t) => t.role + ':' + t.text)`);
  check('the skill text came back to the brain as a tool result', sent.some((t) => t.startsWith('user:[tool result, data] [skill standup]') && t.includes('three questions')));

  await ev(`window.__brain = [${JSON.stringify(ask)}, { say: 'Fine, no notes.' }]; window.__calls.length = 0`);
  await send('again please');
  await press('no');
  check('after No nothing is called', (await skillCalls()) === 0 && await ev(`!window.__calls.some((c) => c[0] === 'mcp_call')`));
  check('after No the slime answers on', (await text()) === 'Fine, no notes.');

  await ev(`window.__brain = [{ error: 'rate_limited' }]`);
  await send('hello');
  check('a rate-limited brain gives a short readable message', (await text()) === 'I am a bit out of breath, try again in a moment.');
  check('the field is back so the user can retry', await ev(`!document.querySelector('#bubble .bubble-form').hidden`));
  await ev(`window.__brain = [{ error: 'provider:Rate limit reached for model' }]`);
  await send('hello again');
  check('a provider message is relayed in a sentence', /Rate limit reached/.test(await text()));
  await ev(`window.__brain = [{ say: 'Back again.' }]`);
  await send('still there?');
  check('the chat recovers after the failure', (await text()) === 'Back again.');

  await page.shot('skill');
  const errors = await page.errors();
  check('no page errors', errors.length === 0);
  if (errors.length) console.log(JSON.stringify(errors));
  page.close();
  console.log(checks.every(Boolean) ? 'RESULT PASS' : 'RESULT FAIL');
  process.exit(checks.every(Boolean) ? 0 : 1);
})();
