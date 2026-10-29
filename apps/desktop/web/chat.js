// A conversation with the brain, in the speech bubble. The history stays in memory only.

import { explain, say } from './i18n.js';
import { canHear } from './voice.js';
import { describeRequest, whenToEpoch } from './reminders.js';

const KEEP_TURNS = 12;
/** Tools the slime may ask for in answer to one message: a budget, so a confused model cannot loop. */
export const STEP_LIMIT = 4;
const SHOW_FACE_FOR = 4.5;

export function createChat({ dialogue, invoke, language, onExpression, isSulking, onMessage, voice = null }) {
  let open = false;
  let waiting = false;
  const history = [];
  let pending = null;
  let steps = 0;
  const confirmField = (request) => ({ confirm: true, placeholder: say(language, 'toolAsk', request), submit: say(language, 'yes'), later: say(language, 'no') });

  const field = () => ({
    placeholder: say(language, 'chatPlaceholder'), submit: say(language, 'send'), later: say(language, 'close'), maxLength: 240,
    ...(voice && canHear(voice.status) ? { mic: () => api.toggleMic(), micLabel: say(language, 'micLabel'), listening: voice.hearing } : {}),
  });

  /** The slime's own powers are read by the backend directly; everything else goes to the MCP server. */
  async function runTool({ server, name, arguments: args }) {
    if (server !== 'slimy') return invoke('mcp_call', { server, tool: name, arguments: args });
    if (name === 'clipboard') return { text: await invoke('clipboard_read'), failed: false };
    if (name === 'active_app') return { text: await invoke('active_app'), failed: false };
    if (name === 'screen') return { text: say(language, 'toolScreen'), image: await invoke('screen_capture'), failed: false };
    if (name === 'skill') {
      const body = await invoke('skill_read', { name: String(args?.name ?? '') });
      return { text: `[skill ${args?.name}]\n${body}`, failed: false };
    }
    if (name === 'remember') {
      await invoke('soul_learn', { key: String(args?.key ?? ''), value: String(args?.value ?? '') });
      return { text: say(language, 'toolRemembered'), failed: false };
    }
    if (name === 'remind') {
      const when = whenToEpoch(args);
      if (!when) throw 'reminder_when';
      await invoke('reminder_add', { at: when.at, text: String(args?.text ?? ''), everyDay: args?.every_day === true });
      return { text: say(language, 'toolReminded', { when: when.label }), failed: false };
    }
    if (name === 'pointer') {
      const whole = (v) => Math.round(Number(v) || 0);
      await invoke('pointer_nudge', { dx: whole(args?.dx), dy: whole(args?.dy) });
      return { text: say(language, 'pointerDone'), failed: false };
    }
    if (name === 'files') return { text: await invoke('files_read', { path: String(args?.path ?? '') }), failed: false };
    throw 'mcp_unknown';
  }

  /** Asks the brain, shows its words, and turns a tool request into a question for the user. */
  async function speak() {
    const reply = await invoke('brain_chat', { history, sulking: !!isSulking() });
    history.push({ role: 'assistant', text: reply.say });
    if (reply.expression) onExpression(reply.expression, SHOW_FACE_FOR);
    if (reply.tool && steps >= STEP_LIMIT) {
      pending = null;
      dialogue.show({ message: say(language, 'stepLimit', { n: STEP_LIMIT }), field: field() });
    } else if (reply.tool) {
      steps += 1;
      pending = reply.tool;
      const warning = reply.tool.name === 'screen' ? ` ${say(language, 'screenWarn')}` : '';
      const detail = describeRequest(reply.tool);
      dialogue.show({ message: `${reply.say}\n${say(language, 'toolAsk', { server: reply.tool.server, name: reply.tool.name })}${detail ? ` ${detail}` : ''}${warning}`, field: confirmField(reply.tool) });
    } else {
      dialogue.show({ message: reply.say, field: field() });
    }
    // The slime says its answer out loud when it has a voice (never blocks: a missing voice is silence).
    voice?.say(reply.say, reply);
  }

  const api = {
    get open() { return open; },
    start(greeting = say(language, 'chatGreeting')) {
      open = true;
      dialogue.show({ message: greeting, field: field() });
    },
    /** The microphone button: starts listening, or stops and uses what was heard so far. */
    async toggleMic() {
      if (!voice || !open || waiting) return;
      if (voice.hearing) { voice.stopHearing(); return; }
      voice.engaged();
      voice.stopSpeaking();
      steps = 0;
      dialogue.show({ message: say(language, 'listening'), field: { ...field(), listening: true } });
      try {
        const heard = await voice.hear();
        if (open) await api.answer(heard);
      } catch (error) {
        const message = String(error) === 'no_permission' ? say(language, 'micNeedsPower') : explain(language, error);
        if (open) dialogue.show({ message, field: field() });
      }
    },
    close() {
      voice?.cut();
      voice?.stopSpeaking();
      voice?.stopHearing();
      open = false;
      pending = null;
      dialogue.hide();
    },
    get asking() { return !!pending; },
    /** The user refused the tool the slime asked for. */
    async decline() {
      if (!pending) return;
      pending = null;
      await this.continue(say(language, 'toolDeclined'));
    },
    /** Runs the confirmed tool, then lets the brain read what came back. */
    async approve() {
      const request = pending;
      if (!request || waiting) return null;
      pending = null;
      waiting = true;
      dialogue.show({ message: say(language, 'toolRunning', { name: request.name }), field: null });
      let result, image;
      try {
        const out = await runTool(request);
        result = out.image ? out.text : say(language, out.failed ? 'toolFailed' : 'toolResult', { text: out.text });
        image = out.image;
      } catch (error) {
        result = say(language, 'toolFailed', { text: explain(language, error) });
      } finally {
        waiting = false;
      }
      await this.continue(result, image);
      return null;
    },
    /** Adds a tool outcome to the history as the user's turn and gets the brain's next words. */
    async continue(text, image) {
      // Only the newest picture travels with the conversation: an older one would make every request heavier.
      if (image) history.forEach((turn) => { delete turn.image; });
      history.push(image ? { role: 'user', text, image } : { role: 'user', text });
      waiting = true;
      dialogue.show({ message: say(language, 'thinking'), field: field() });
      try {
        await speak();
      } catch (error) {
        history.pop();
        dialogue.show({ message: explain(language, error), field: field() });
      } finally {
        waiting = false;
      }
    },
    /** The text typed in the bubble. Returns an error sentence for the field, or null. */
    async answer(text) {
      const value = String(text ?? '').trim();
      if (!open || waiting) return null;
      if (pending) return this.approve();
      if (!value) return say(language, 'emptyAnswer');
      steps = 0;
      voice?.engaged();
      voice?.stopSpeaking();
      history.push({ role: 'user', text: value });
      onMessage?.();
      if (history.length > KEEP_TURNS) history.splice(0, history.length - KEEP_TURNS);
      waiting = true;
      dialogue.show({ message: say(language, 'thinking'), field: field() });
      try {
        await speak();
      } catch (error) {
        history.pop();
        dialogue.show({ message: explain(language, error), field: field() });
      } finally {
        waiting = false;
      }
      return null;
    },
  };
  return api;
}
