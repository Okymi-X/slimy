// The whole birth, on the real page (dev/app-harness.html?nosoul=1, soul creation faked): the egg wobbles, is poked until it
// hatches, asks for two names through the bubble, and the life begins. Real mouse and keyboard events.
// Usage: node birth-test.mjs [provider] [port]
import { launch, sleep } from './cdp.mjs';

const [provider = 'groq', port = '9430'] = process.argv.slice(2);

(async () => {
  const page = await launch({ provider, port, tag: `birth-${provider}`, soul: '&nosoul=1' });
  await sleep(6000);
  const bubble = () => page.evaluate(`(() => { const b = document.getElementById('bubble'); const i = b.querySelector('input');
    return { hidden: b.hidden, text: (b.querySelector('p') || {}).textContent || '', input: !!i && !b.querySelector('form').hidden }; })()`);
  const egg = await page.shot('0-egg');
  console.log(`egg drawn: box ${egg.box} (${egg.box[2] - egg.box[0]}x${egg.box[3] - egg.box[1]}); page errors ${JSON.stringify(await page.errors())}`);
  if (egg.box[2] < 0) { console.log('FAIL: nothing is drawn'); page.close(); process.exit(1); }
  const [cx, cy] = egg.pts.center;
  let asked = null, pokes = 0;
  for (let i = 0; i < 20; i++) {
    await page.mouse('mousePressed', cx, cy); await sleep(80); await page.mouse('mouseReleased', cx, cy); pokes++;
    await sleep(900);
    const state = await bubble();
    if (state.input) { asked = state; break; }
    if (i === 2) await page.shot('1-cracking');
  }
  if (!asked) { console.log(`FAIL: the egg never asked for a name after ${pokes} pokes`); page.close(); process.exit(1); }
  console.log(`hatched after ${pokes} pokes; first question: "${asked.text}"`);
  await page.shot('2-first-question');
  const type = async (text) => {
    await page.call('Input.insertText', { text });
    await page.call('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r' });
    await page.call('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
    await sleep(1800);
  };
  await type('Testeur');
  const second = await bubble();
  console.log(`second question: "${second.text}" (input ${second.input})`);
  await page.shot('3-second-question');
  await type('Gloo');
  await sleep(2500);
  const welcome = await bubble();
  await page.shot('4-welcome');
  const calls = await page.evaluate('window.__calls.filter((c) => c[0] === "soul_create").map((c) => c[1])');
  console.log(`soul_create called with ${JSON.stringify(calls)}; bubble after: "${welcome.text}"`);
  await sleep(9000);
  const life = await page.shot('5-life');
  const reloaded = await page.evaluate('window.__calls.filter((c) => c[0] === "soul_load").length');
  const errors = await page.errors();
  console.log(`after the welcome: slime box ${life.box}, soul_load called ${reloaded} times, page errors ${JSON.stringify(errors)}`);
  const ok = calls.length === 1 && calls[0].userName === 'Testeur' && calls[0].slimeName === 'Gloo' && second.input && second.text !== asked.text && reloaded >= 2 && life.box[2] > 0 && !errors.length;
  console.log(`RESULT ${ok ? 'PASS' : 'FAIL'}`);
  page.close();
  process.exit(ok ? 0 : 1);
})().catch((e) => { console.error('test crashed:', e); process.exit(2); });
