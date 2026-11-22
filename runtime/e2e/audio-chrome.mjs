// Full-stack E2E (manual) of rwire-audio in a real Chrome: the shipped
// extension and worklet, a real microphone pipeline (Chrome's fake
// device), real playback, against a LIVE `examples/audio` server.
//
//   cargo run -p audio                      # serves http://127.0.0.1:9012
//   node e2e/audio-chrome.mjs [chrome]      # from runtime/, Node 22+
//
// Chrome runs headless with `--use-fake-device-for-media-stream` and
// `--use-file-for-fake-audio-capture=<silence.wav>` (written here), and
// `--use-fake-ui-for-media-stream` to grant the microphone. Clicks are
// real input events (CDP `Input.dispatchMouseEvent`), so they are user
// gestures: the extension resumes its audio context in them, with no
// autoplay flag.
//
// Checks, read from the page (`#measure` and the event lines):
// 1. the microphone on: capture Granted, output Ready, frames flow;
// 2. a 2 s tone with 4 marks: PlaybackStarted, MarkReached t1..t4 about
//    0.5 s apart on the audio clock, PlaybackEnded; and the loudest
//    microphone frame during it stays under ECHO_MAX (the echo check);
// 3. the tone again, "Stop" after 0.6 s: Stopped with last mark t1, and the
//    silence within 300 ms of the order, on the audio clock;
// 4. the microphone off: the page plays back what it heard (a mark "echo"
//    and an end).
//
// With the fake device there is no acoustic path from the speakers to the
// microphone, so check 2 proves that nothing of the output leaks into the
// captured frames inside the pipeline. Echo cancellation itself needs
// speakers and a microphone in a room: see the manual recipe in the
// rwire-audio docs.

import { spawn } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const CHROME = process.argv[2] || "google-chrome";
const URL = process.env.AUDIO_URL || "http://127.0.0.1:9012/";
const PORT = 9333;
const ECHO_MAX = 0.01;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const dir = mkdtempSync(join(tmpdir(), "rw-audio-e2e-"));

// One second of silence, 48 kHz mono PCM16.
function silenceWav(rate = 48000, seconds = 1) {
  const n = rate * seconds;
  const b = Buffer.alloc(44 + n * 2);
  b.write("RIFF", 0);
  b.writeUInt32LE(36 + n * 2, 4);
  b.write("WAVEfmt ", 8);
  b.writeUInt32LE(16, 16);
  b.writeUInt16LE(1, 20);
  b.writeUInt16LE(1, 22);
  b.writeUInt32LE(rate, 24);
  b.writeUInt32LE(rate * 2, 28);
  b.writeUInt16LE(2, 32);
  b.writeUInt16LE(16, 34);
  b.write("data", 36);
  b.writeUInt32LE(n * 2, 40);
  return b;
}
const wav = join(dir, "silence.wav");
writeFileSync(wav, silenceWav());

const chrome = spawn(
  CHROME,
  [
    "--headless=new",
    `--remote-debugging-port=${PORT}`,
    `--user-data-dir=${join(dir, "profile")}`,
    "--no-first-run",
    "--use-fake-device-for-media-stream",
    "--use-fake-ui-for-media-stream",
    `--use-file-for-fake-audio-capture=${wav}`,
    "about:blank",
  ],
  { stdio: "ignore" },
);

let failed = 0;
const check = (ok, what) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${what}`);
  if (!ok) failed++;
};

try {
  let targets;
  for (let k = 0; k < 50 && !targets; k++) {
    await sleep(200);
    targets = await fetch(`http://127.0.0.1:${PORT}/json/list`).then((r) => r.json()).catch(() => null);
  }
  const page = targets.find((t) => t.type === "page");
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((r) => (ws.onopen = r));
  let id = 0;
  const waiting = new Map();
  ws.onmessage = (m) => {
    const msg = JSON.parse(m.data);
    if (msg.id && waiting.has(msg.id)) {
      waiting.get(msg.id)(msg);
      waiting.delete(msg.id);
    }
  };
  const cdp = (method, params = {}) =>
    new Promise((r) => {
      const n = ++id;
      waiting.set(n, r);
      ws.send(JSON.stringify({ id: n, method, params }));
    });
  const evaluate = async (expr) => (await cdp("Runtime.evaluate", { expression: expr, returnByValue: true })).result.result.value;
  /** A real click (a user gesture) on the button whose text starts with `label`. */
  const click = async (label) => {
    const box = await evaluate(`(() => {
      const b = [...document.querySelectorAll("button")].find((b) => b.textContent.trim().startsWith(${JSON.stringify(label)}));
      if (!b) return null;
      const r = b.getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
    })()`);
    if (!box) throw new Error(`no button "${label}"`);
    for (const type of ["mousePressed", "mouseReleased"])
      await cdp("Input.dispatchMouseEvent", { type, x: box.x, y: box.y, button: "left", clickCount: 1 });
  };
  const measure = async () => {
    const text = await evaluate(`document.querySelector("#measure")?.textContent || ""`);
    return Object.fromEntries(text.split(" ").map((kv) => kv.split("=")));
  };
  const events = () => evaluate(`[...document.querySelectorAll("#measure ~ p")].map((p) => p.textContent)`);
  const num = (line, key) => Number((line.match(new RegExp(key + ": ([0-9.e-]+)")) || [])[1]);

  await cdp("Page.enable");
  await cdp("Page.navigate", { url: URL });
  await sleep(1500);

  // 1. Microphone on.
  await click("Microphone on");
  await sleep(1500);
  let m = await measure();
  check(m.capture === "Granted", `capture Granted (${m.capture})`);
  check(m.output === "Ready", `output Ready (${m.output})`);
  check(Number(m.frames) >= 40, `frames flow (${m.frames} in ~1.5 s)`);

  // 2. A tone with marks; the echo check.
  await click("Play a tone");
  await sleep(3000);
  let ev = await events();
  const started = ev.filter((e) => e.startsWith("PlaybackStarted"));
  const marks = ev.filter((e) => e.startsWith("MarkReached"));
  const ended = ev.filter((e) => e.startsWith("PlaybackEnded"));
  check(started.length >= 1 && ended.length >= 1, "PlaybackStarted then PlaybackEnded");
  check(marks.map((e) => (e.match(/id: "(t\d)"/) || [])[1]).join(",") === "t1,t2,t3,t4", "marks t1..t4 in order");
  const times = [num(started.at(-1), "at"), ...marks.map((e) => num(e, "at")), num(ended.at(-1), "at")];
  const gaps = times.slice(1).map((t, k) => t - times[k]);
  console.log("     audio-clock times:", times.map((t) => t.toFixed(3)).join(" "));
  check(gaps.slice(0, 4).every((g) => Math.abs(g - 0.5) < 0.03), `marks 0.5 s apart (${gaps.map((g) => g.toFixed(3)).join(", ")})`);
  m = await measure();
  check(m.playmax !== "-" && Number(m.playmax) < ECHO_MAX, `echo: loudest frame during playback ${m.playmax} < ${ECHO_MAX}`);

  // 3. Stop.
  await click("Play a tone");
  await sleep(600);
  await click("Stop");
  await sleep(1000);
  ev = await events();
  const stopped = ev.filter((e) => e.startsWith("Stopped")).at(-1) || "";
  check(/last_mark: Some\("t1"\)/.test(stopped), `Stopped after t1 (${stopped})`);
  m = await measure();
  check(m.stopms !== "-" && Number(m.stopms) <= 300, `stop heard ${m.stopms} ms after the order (≤ 300)`);

  // 4. Microphone off: played back.
  await click("Microphone off");
  // What was heard since the microphone went on (a few seconds) plays back.
  let echoed = false;
  for (let k = 0; k < 50 && !echoed; k++) {
    await sleep(200);
    echoed = (await events()).some((e) => e.includes('id: "echo"'));
  }
  check(echoed, "what was heard is played back (mark echo)");
  check((await measure()).capture === "Granted", "state kept");
  if (process.env.DEBUG) console.log((await events()).join("\n"));
} catch (e) {
  console.log("FAIL", e);
  failed++;
} finally {
  chrome.kill();
}
console.log(failed ? `${failed} failed` : "all passed");
process.exit(failed ? 1 : 0);
