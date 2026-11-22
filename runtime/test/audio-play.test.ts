// Audio extension, playback side: the gapless player (event order, marks
// at their sample, underrun before the end, stop dropping the queue), the
// resampler, then the extension with the real `rw-play` worklet on fake
// audio-thread globals: server messages in, samples and report lines out,
// latency-compensated times, a stop heard within 300 ms, sounds queued in
// order with the PCM.
import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { makeDom, type MockDoc, type MockEl } from "./dom.ts";
import { Player, Resampler, type Note } from "../src/ext/audio-play.ts";
import { Stream, type Env, type CtxLike, type NodeLike, type SockLike } from "../src/ext/audio.ts";

const QUANTUM = 128;

/** Render `quanta` quanta from output frame `frame0`; the samples and notes. */
function run(p: Player, quanta: number, frame0 = 0) {
  const samples: number[] = [];
  const notes: Note[] = [];
  for (let q = 0; q < quanta; q++) {
    const out = new Float32Array(QUANTUM);
    notes.push(...p.render(out, frame0 + q * QUANTUM));
    samples.push(...out);
  }
  return { samples, notes };
}

const ramp = (n: number, from = 1) => Float32Array.from({ length: n }, (_, k) => (from + k) / 1e5);

test("events come in order: started, the mark at its exact sample, ended after the end", () => {
  const p = new Player();
  p.push({ k: "pcm", data: ramp(1000), ratio: 0.5 });
  p.push({ k: "mark", id: "s1" });
  p.push({ k: "pcm", data: ramp(500, 1001), ratio: 0.5 });
  p.push({ k: "end" });
  const { samples, notes } = run(p, 20, 4096);
  assert.deepEqual(notes, [
    { e: "started", at: 4096 },
    { e: "mark", at: 4096 + 1000, id: "s1" },
    { e: "ended", at: 4096 + 1500 },
  ]);
  // Gapless: the 1500 samples back to back, then silence.
  assert.deepEqual(samples.slice(0, 1500), [...ramp(1500)].map((x) => Math.fround(x)));
  assert.ok(samples.slice(1500).every((x) => x === 0));
});

test("a queue run dry before the end is an underrun, not an end; more PCM resumes without a new start", () => {
  const p = new Player();
  p.push({ k: "pcm", data: ramp(200), ratio: 1 });
  let r = run(p, 4);
  assert.deepEqual(r.notes, [
    { e: "started", at: 0 },
    { e: "underrun", at: 200 },
  ]);
  r = run(p, 2, 512);
  assert.deepEqual(r.notes, [], "one underrun per dry spell");
  p.push({ k: "pcm", data: ramp(100), ratio: 1 });
  p.push({ k: "end" });
  r = run(p, 2, 768);
  assert.deepEqual(r.notes, [{ e: "ended", at: 868 }]);
});

test("stop drops the queue at once and says the last mark and the samples played since", () => {
  const p = new Player();
  p.push({ k: "mark", id: "s1" });
  p.push({ k: "pcm", data: ramp(1200), ratio: 0.5 }); // 24 kHz into 48 kHz
  p.push({ k: "mark", id: "s2" });
  p.push({ k: "pcm", data: ramp(4800), ratio: 0.5 });
  p.push({ k: "end" });
  run(p, 15); // 1920 output samples: s2 reached at 1200, 720 since
  const notes = p.stop(1920);
  assert.deepEqual(notes, [{ e: "stopped", at: 1920, mark: "s2", since: 360 }]);
  const after = run(p, 3, 1920);
  assert.ok(after.samples.every((x) => x === 0), "nothing plays after a stop");
  assert.deepEqual(after.notes, []);
  assert.deepEqual(p.stop(2304), [], "nothing to stop");
});

test("the resampler is continuous across chunks and keeps the duration", () => {
  const src = Int16Array.from({ length: 2400 }, (_, k) => Math.round(8000 * Math.sin(k / 7)));
  const whole = new Resampler(24000, 48000).push(src);
  const r = new Resampler(24000, 48000);
  const parts = [...r.push(src.subarray(0, 1000)), ...r.push(src.subarray(1000, 1001)), ...r.push(src.subarray(1001))];
  assert.equal(parts.length, whole.length);
  parts.forEach((x, k) => assert.ok(Math.abs(x - whole[k]) < 1e-6, `sample ${k}`));
  assert.ok(Math.abs(whole.length - 4800) <= 2, `${whole.length} samples for 50 ms`);
  // 22.05 kHz too.
  const r2 = new Resampler(22050, 48000);
  let n = 0;
  for (let k = 0; k < 22050; k += 441) n += r2.push(new Int16Array(441)).length;
  assert.ok(Math.abs(n - 48000) <= 3, `${n} samples for one second`);
});

// --- the extension with the real worklet ---

const g = globalThis as any;
const registry: Record<string, any> = {};
g.sampleRate = 48000;
g.currentFrame = 0;
g.AudioWorkletProcessor = class {
  port: any = { postMessage: (m: any) => this.port.deliver && this.port.deliver(m), onmessage: null };
};
g.registerProcessor = (name: string, ctor: unknown) => (registry[name] = ctor);
await import("../src/ext/audio-worklet.ts");

interface Rig {
  env: Env;
  sock: FakeSock;
  ctx: CtxLike;
  proc: () => any;
  timers: { f: () => void; ms: number }[];
  /** Render `quanta` quanta of the play node; its samples. */
  render(quanta: number): number[];
  /** Fire the report timers that are due (all of them). */
  flush(): void;
  fetches: ((b: ArrayBuffer) => void)[];
}

class FakeSock implements SockLike {
  binaryType = "blob";
  readyState = 1;
  onopen: (() => void) | null = null;
  onmessage: ((m: { data: ArrayBuffer | string }) => void) | null = null;
  onclose: ((e: { code: number }) => void) | null = null;
  send() {}
  close() {}
  serverSends(bytes: Uint8Array) {
    this.onmessage && this.onmessage({ data: bytes.slice().buffer });
  }
}

let doc: MockDoc;
let el: MockEl;
let reports: string[];

beforeEach(() => {
  doc = makeDom().document;
  reports = [];
  g.currentFrame = 0;
  el = doc.createElement("div");
  el.setAttribute("data-audio", "voice");
  el.setAttribute("data-audio-ticket", "rwt_a");
  el.setAttribute("data-audio-sounds", JSON.stringify([["ding", "/sounds/ding.wav"]]));
  const input = doc.createElement("input");
  input.setAttribute("data-audio-report", "1");
  // Playback lines only: the output state has its own tests.
  (input as any).dispatchEvent = () => input.value.startsWith("output") || reports.push(input.value);
  el.appendChild(input);
  doc.body.appendChild(el);
});

function rig(): Rig {
  const timers: { f: () => void; ms: number }[] = [];
  const procs: any[] = [];
  const fetches: ((b: ArrayBuffer) => void)[] = [];
  let sock!: FakeSock;
  const ctx: CtxLike = {
    state: "running",
    get currentTime() {
      return g.currentFrame / 48000;
    },
    sampleRate: 48000,
    outputLatency: 0.04,
    baseLatency: 0.01,
    destination: {},
    onstatechange: null,
    audioWorklet: { addModule: () => Promise.resolve() },
    resume: () => Promise.resolve(),
    close: () => Promise.resolve(),
    createMediaStreamSource: () => ({ port: { onmessage: null, postMessage() {} }, connect() {}, disconnect() {} }),
    // The ding: 64 samples of 0.25.
    decodeAudioData: () => Promise.resolve({ getChannelData: () => new Float32Array(64).fill(0.25) }),
  };
  const env: Env = {
    secure: true,
    media: undefined,
    context: () => ctx,
    node: (_c, name) => {
      const proc = new registry[name]();
      procs.push(proc);
      const n: NodeLike = {
        port: { onmessage: null, postMessage: (m: any) => proc.port.onmessage({ data: m }) },
        connect() {},
        disconnect() {},
      };
      proc.port.deliver = (m: any) => n.port.onmessage && n.port.onmessage({ data: m });
      return n;
    },
    socket: () => (sock = new FakeSock()),
    origin: "ws://host",
    worklet: "/w.js",
    later: (f, ms) => timers.push({ f, ms }),
    // The sound loads slowly: resolved by the test.
    fetch: () => Promise.resolve({ arrayBuffer: () => new Promise<ArrayBuffer>((r) => fetches.push(r)) }),
  };
  const s = new Stream(el as unknown as Element, env);
  s.context();
  s.sync();
  return {
    env,
    get sock() {
      return sock;
    },
    ctx,
    proc: () => procs.find((p) => p.constructor === registry["rw-play"]),
    timers,
    fetches,
    render(quanta: number) {
      const out: number[] = [];
      for (let q = 0; q < quanta; q++) {
        const block = new Float32Array(QUANTUM);
        const p = procs.find((p) => p.constructor === registry["rw-play"]);
        if (p) p.process([], [[block]]);
        out.push(...block);
        g.currentFrame += QUANTUM;
      }
      return out;
    },
    flush() {
      while (timers.length) timers.shift()!.f();
    },
  } as Rig;
}

const settle = async () => {
  for (let k = 0; k < 10; k++) await new Promise((r) => setTimeout(r, 0));
};
const msg = (tag: number, body: Uint8Array = new Uint8Array(0)) => Uint8Array.from([tag, ...body]);
const text = (s: string) => new TextEncoder().encode(s);
function pcmMsg(rate: number, samples: number[]) {
  const b = new Uint8Array(5 + samples.length * 2);
  const v = new DataView(b.buffer);
  v.setUint8(0, 0x81);
  v.setUint32(1, rate, true);
  samples.forEach((x, k) => v.setInt16(5 + 2 * k, x, true));
  return b;
}

test("server PCM at 24 kHz plays at the context's rate; started, mark and ended are reported when heard", async () => {
  const r = rig();
  await settle();
  r.sock.serverSends(pcmMsg(24000, new Array(240).fill(8192))); // 10 ms
  r.sock.serverSends(msg(0x82, text("sentence 1")));
  r.sock.serverSends(pcmMsg(24000, new Array(240).fill(8192)));
  r.sock.serverSends(msg(0x85));
  await settle();
  const out = r.render(10);
  const loud = out.filter((x) => x !== 0).length;
  assert.ok(Math.abs(loud - 960) <= 2, `20 ms at 48 kHz: ${loud} samples`);
  // Lines are written when heard: latency 50 ms, and each timer waits for it.
  assert.deepEqual(reports, []);
  const waits = r.timers.map((t) => Math.round(t.ms));
  r.flush();
  const lines = reports.map((l) => l.split(" "));
  assert.deepEqual(
    lines.map((l) => l[0]),
    ["started", "mark", "ended"],
  );
  assert.equal(lines[1][2] + " " + lines[1][3], "sentence 1");
  const t = lines.map((l) => Number(l[1]));
  assert.ok(Math.abs(t[0] - 0.05) < 1e-6, `started at 0 + 50 ms latency: ${t[0]}`);
  assert.ok(Math.abs(t[1] - (480 / 48000 + 0.05)) < 1e-4, `mark after 10 ms: ${t[1]}`);
  assert.ok(Math.abs(t[2] - (960 / 48000 + 0.05)) < 1e-4, `ended after 20 ms: ${t[2]}`);
  // Each line waited until its sample was heard (rendered + latency, from the render clock).
  assert.equal(waits[0], 50, "started rendered at t=0 is written 50 ms later, when heard");
});

test("stop drops the queue: silence from the next quantum, Stopped heard within 300 ms", async () => {
  const r = rig();
  await settle();
  r.sock.serverSends(msg(0x82, text("s1")));
  r.sock.serverSends(pcmMsg(24000, new Array(24000).fill(8192))); // one second
  r.sock.serverSends(msg(0x85));
  await settle();
  r.render(100); // ~267 ms played
  const stopAt = g.currentFrame / 48000;
  r.sock.serverSends(msg(0x84));
  await settle();
  const after = r.render(4);
  assert.ok(after.every((x) => x === 0), "silent from the next render quantum");
  r.flush();
  const stopped = reports.find((l) => l.startsWith("stopped"))!.split(" ");
  const heardSilent = Number(stopped[1]);
  assert.ok(heardSilent - stopAt <= 0.3, `stop heard ${((heardSilent - stopAt) * 1000).toFixed(1)} ms after the order`);
  assert.equal(stopped[3], "s1", "the last mark reached");
  assert.equal(Number(stopped[2]), (100 * QUANTUM) / 2, "24 kHz samples played since it");
  assert.ok(!reports.some((l) => l.startsWith("ended")), "a stopped utterance does not end");
});

test("a stop drops what is still waiting for a slow sound too", async () => {
  const r = rig();
  await settle();
  r.sock.serverSends(msg(0x83, text("ding")));
  r.sock.serverSends(pcmMsg(24000, new Array(240).fill(8192)));
  r.sock.serverSends(msg(0x84));
  r.fetches.forEach((f) => f(new ArrayBuffer(8)));
  await settle();
  assert.ok(r.render(4).every((x) => x === 0));
});

test("sounds are queued in call order with the PCM, even when they load last", async () => {
  const r = rig();
  await settle();
  r.sock.serverSends(msg(0x83, text("ding")));
  r.sock.serverSends(pcmMsg(48000, new Array(64).fill(16384)));
  r.sock.serverSends(msg(0x83, text("ding")));
  r.sock.serverSends(msg(0x85));
  await settle();
  assert.deepEqual(r.render(2).filter((x) => x !== 0), [], "nothing before the ding is loaded");
  r.fetches.forEach((f) => f(new ArrayBuffer(8)));
  await settle();
  const out = r.render(2).slice(0, 192);
  assert.deepEqual(out.slice(0, 64), new Array(64).fill(0.25), "the ding first");
  assert.deepEqual(out.slice(64, 128), new Array(64).fill(0.5), "then the PCM");
  assert.deepEqual(out.slice(128, 192), new Array(64).fill(0.25), "then the ding again");
  r.flush();
  assert.deepEqual(
    reports.map((l) => l.split(" ")[0]),
    ["started", "ended"],
  );
});
