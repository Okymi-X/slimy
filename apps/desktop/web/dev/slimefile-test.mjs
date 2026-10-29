// Soul export and import menu entries on the real page (dev/app-harness.html, backend faked).
// Needs `python3 -m http.server 9742` in apps/desktop/web and HARNESS_PORT=9742.
import { launch, sleep } from './cdp.mjs';

(async () => {
  const page = await launch({ provider: 'groq', port: '9743', tag: 'slimefile' });
  await sleep(5000);
  const checks = [];
  const check = (name, ok) => { checks.push(ok); console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}`); };
  const ev = (code) => page.evaluate(code);
  const item = (id) => ev(`window.__menu.find((i) => i.id === '${id}').action()`);
  const bubble = () => ev(`document.querySelector('#bubble')?.textContent ?? ''`);
  const calls = (name) => ev(`window.__calls.filter((c) => c[0] === '${name}').length`);
  const click = (n) => ev(`document.querySelectorAll('#bubble .bubble-form button:not([hidden])')[${n}].click()`);
  const reset = (slime = {}) => ev(`(window.__calls.length = 0, window.__slime = ${JSON.stringify(slime)}, undefined)`);

  await reset();
  await item('slime-export'); await sleep(600);
  check('export calls the backend and shows the path', (await calls('slime_export')) === 1 && (await bubble()).includes('/home/u/exports/slimy.slime'));

  await reset({ exportError: 'slime_too_large' });
  await item('slime-export'); await sleep(600);
  check('an export error is a readable message', /too big|trop gros/.test(await bubble()));

  await reset();
  await item('slime-import'); await sleep(700);
  const text = await bubble();
  check('the preview asks with name, nights and memories', /Blob/.test(text) && /12/.test(text) && /3 (memories|souvenirs)/.test(text) && /slimy\.slime/.test(text));
  check('the bubble offers Yes and No', (await ev(`document.querySelectorAll('#bubble .bubble-form button:not([hidden])').length`)) === 2);
  await page.shot('ask');
  await click(1); await sleep(500);
  check('No applies nothing', (await calls('slime_import_apply')) === 0 && /nothing changes|ne change rien/.test(await bubble()));

  await reset({ previewError: 'slime_missing' });
  await item('slime-import'); await sleep(600);
  check('a preview error is readable and asks nothing', /no \.slime|aucun fichier/.test(await bubble()) && (await calls('slime_import_apply')) === 0);

  const garbage = [
    ['a path as file name', { file: '../../etc/passwd.slime', slime_name: 'x', nights: 1, facts: 1 }],
    ['a negative count', { file: 'a.slime', slime_name: 'x', nights: -1, facts: 1 }],
    ['a text count', { file: 'a.slime', slime_name: 'x', nights: '<b>9</b>', facts: 1 }],
    ['a missing name', { file: 'a.slime', nights: 1, facts: 1 }],
    ['null', null],
  ];
  for (const [label, preview] of garbage) {
    await reset({ preview });
    await item('slime-import'); await sleep(500);
    const noQuestion = await ev(`document.querySelector('#bubble .bubble-form').hidden`);
    check(`garbage preview (${label}) never reaches the bubble`, /unreadable|illisible/.test(await bubble()) && noQuestion);
  }

  await reset({ preview: { file: 'a.slime', slime_name: '<img src=x onerror=alert(1)>aaaaaaaaaaaaaaaaaaaaaaaa', nights: 1, facts: 0 } });
  await item('slime-import'); await sleep(500);
  check('a hostile name is cut and shown as text only', (await ev(`document.querySelector('#bubble img')`)) === null && (await bubble()).includes('<img src=x'));
  await click(1); await sleep(300);

  await reset({ applyError: 'slime_bad_version' });
  await item('slime-import'); await sleep(600);
  await ev(`window.__marker = 1`);
  await click(0); await sleep(600);
  check('an apply error is readable', /version/.test(await bubble()) && (await calls('slime_import_apply')) === 1);
  await sleep(2200);
  check('and the page does not reload', (await ev(`window.__marker`)) === 1);

  await reset();
  await item('slime-import'); await sleep(600);
  await click(0); await sleep(700);
  check('Yes applies with the bare file name', await ev(`window.__calls.some((c) => c[0] === 'slime_import_apply' && c[1].file === 'slimy.slime' && Object.keys(c[1]).length === 1)`));
  check('Yes says it restarts', /Done|C'est fait/.test(await bubble()));
  check('the page has not reloaded yet', (await ev(`window.__marker`)) === 1);
  await sleep(2200);
  check('then the page reloads', (await ev(`window.__marker`)) === undefined);

  const errors = await page.errors();
  check('no page errors', errors.length === 0);
  if (errors.length) console.log(JSON.stringify(errors));
  page.close();
  console.log(checks.every(Boolean) ? 'RESULT PASS' : 'RESULT FAIL');
  process.exit(checks.every(Boolean) ? 0 : 1);
})();
