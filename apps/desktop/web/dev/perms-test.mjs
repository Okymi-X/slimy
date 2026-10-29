// The permissions panel on the real page: opened from the menu, a timed power is granted, the "watching" badge shows,
// the panel lists the access, and revoking hides the badge. Needs `python3 -m http.server 8765` in apps/desktop/web.
import { launch, sleep } from './cdp.mjs';

(async () => {
  const page = await launch({ provider: 'groq', port: '9440', tag: 'perms' });
  await sleep(5000);
  const ui = () => page.evaluate(`(() => ({ open: !document.getElementById('perms').hidden, badge: !document.getElementById('watching').hidden,
    rows: [...document.querySelectorAll('#perms .power-list .power-row')].map((r) => r.firstChild.textContent), buttons: [...document.querySelectorAll('#perms .power-list .power-row')].map((r) => r.querySelectorAll('button').length),
    log: [...document.querySelectorAll('#perms .power-log li')].map((l) => l.textContent) }))()`);
  const click = (expr) => page.evaluate(`(${expr})()`);
  const checks = [];
  const check = (name, ok) => { checks.push(ok); console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}`); };

  await page.evaluate(`window.__menu.find((i) => i.id === 'perms').action()`);
  await sleep(600);
  let s = await ui();
  check('the panel opens with the seven powers', s.open && s.rows.length === 7);
  check('timed powers offer three durations, the others one button', s.buttons.join() === '1,3,3,1,1,3,3');
  check('no badge before any grant', !s.badge);

  await click(`() => [...document.querySelectorAll('#perms .power-list .power-row')][2].querySelectorAll('button')[0].click()`);
  await sleep(500);
  s = await ui();
  check('granting the screen shows the badge and a revoke button', s.badge && s.buttons[2] === 1);
  check('the call carried a duration', (await page.evaluate(`JSON.stringify(window.__calls.filter((c) => c[0] === 'perms_grant'))`)).includes('"seconds":300'));

  await click(`() => [...document.querySelectorAll('#perms .power-list .power-row')][2].querySelectorAll('button')[0].click()`);
  await sleep(500);
  s = await ui();
  check('revoking hides the badge', !s.badge);
  console.log(`page errors ${JSON.stringify(await page.errors())}`);
  check('no page errors', (await page.errors()).length === 0);
  await page.shot('panel');
  page.close();
  console.log(checks.every(Boolean) ? 'RESULT PASS' : 'RESULT FAIL');
  process.exit(checks.every(Boolean) ? 0 : 1);
})();
