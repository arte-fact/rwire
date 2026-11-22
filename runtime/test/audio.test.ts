// Audio extension, capture side: the DSP (16 kHz, 20 ms frames, PCM16),
// the microphone constraints, the four capture states, `capture(false)`
// stopping every track, the output state and its resume on a gesture, and
// the socket (ticket, next ticket, reconnection, refusal). The browser
// surface (getUserMedia, AudioContext, the worklet node, WebSocket) is a
// fake Env.
import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { makeDom, type MockDoc, type MockEl } from "./dom.ts";
import { Capture, encodeFrame, FRAME, RATE, level, toI16 } from "../src/ext/audio-dsp.ts";
import {
  CONSTRAINTS,
  Stream,
  i as install,
  reset,
  current,
  type Env,
  type CtxLike,
  type NodeLike,
  type SockLike,
} from "../src/ext/audio.ts";

// --- DSP ---

const sine = (rate: number, hz: number, n: number, from = 0, amp = 0.5) =>
  Float32Array.from({ length: n }, (_, k) => amp * Math.sin((2 * Math.PI * hz * (from + k)) / rate));

/** Feed `seconds` of a 1 kHz sine at `rate` in 128-sample blocks; the frames. */
function capture(rate: number, seconds: number, frame0 = 0) {
  const c = new Capture(rate);
  const frames = [];
  const total = Math.round(rate * seconds);
  for (let at = 0; at < total; at += 128) {
    const n = Math.min(128, total - at);
    frames.push(...c.push(sine(rate, 1000, n, at), frame0 + at));
  }
  return frames;
}

for (const rate of [48000, 44100, 16000, 8000]) {
  test(`${rate} Hz in: 20 ms frames of 320 PCM16 samples at 16 kHz, numbered, timed on the audio clock`, () => {
    // One second, plus a block: interpolation waits for the sample after.
    const all = capture(rate, 1 + 128 / rate, rate * 10);
    assert.equal(all.length, 50, "one second is 50 frames");
    const frames = all;
    frames.forEach((f, k) => {
      assert.equal(f.seq, k);
      assert.ok(f.pcm instanceof Int16Array);
      assert.equal(f.pcm.length, FRAME);
      // Timestamps on the input clock: 10 s (frame0) + k × 20 ms.
      assert.ok(Math.abs(f.t - (10 + k * 0.02)) < 1 / RATE + 1 / rate, `t of frame ${k}: ${f.t}`);
    });
    // The 1 kHz tone keeps its amplitude (0.5 → RMS ≈ 0.354).
    const rms = level(frames[25].pcm);
    assert.ok(Math.abs(rms - 0.5 / Math.SQRT2) < 0.02, `rms ${rms}`);
  });
}

test("a tone above 8 kHz is attenuated by the area averaging (no full-level fold back)", () => {
  const c = new Capture(48000);
  const frames = c.push(sine(48000, 12000, 48000), 0);
  const rms = level(frames[10].pcm);
  assert.ok(rms < 0.2, `12 kHz folded back at rms ${rms}`);
});

test("samples are clamped to PCM16", () => {
  assert.equal(toI16(2), 32767);
  assert.equal(toI16(-2), -32768);
  assert.equal(toI16(0), 0);
});

test("a frame on the wire: 0x01, seq u32, t f64, 320 × i16, little-endian, 653 bytes", () => {
  const pcm = new Int16Array(FRAME);
  pcm[0] = -2;
  pcm[319] = 32767;
  const buf = encodeFrame({ seq: 7, t: 1.25, pcm });
  assert.equal(buf.byteLength, 653);
  const v = new DataView(buf);
  assert.equal(v.getUint8(0), 0x01);
  assert.equal(v.getUint32(1, true), 7);
  assert.equal(v.getFloat64(5, true), 1.25);
  assert.equal(v.getInt16(13, true), -2);
  assert.equal(v.getInt16(13 + 319 * 2, true), 32767);
});

// --- the extension against a fake browser ---

class FakeSock implements SockLike {
  binaryType = "blob";
  readyState = 0;
  onopen: (() => void) | null = null;
  onmessage: ((m: { data: ArrayBuffer | string }) => void) | null = null;
  onclose: ((e: { code: number }) => void) | null = null;
  sent: ArrayBuffer[] = [];
  closed: number | null = null;
  readonly url: string;
  constructor(url: string) {
    this.url = url;
  }
  send(d: ArrayBuffer) {
    this.sent.push(d);
  }
  close(code?: number) {
    this.closed = code ?? 1000;
  }
  open() {
    this.readyState = 1;
    this.onopen && this.onopen();
  }
  serverSends(bytes: number[] | Uint8Array) {
    this.onmessage && this.onmessage({ data: Uint8Array.from(bytes).buffer });
  }
  drop(code: number) {
    this.readyState = 3;
    this.onclose && this.onclose({ code });
  }
}

const ticketMsg = (t: string) => [0x80, ...new TextEncoder().encode(t)];

interface Fake extends Env {
  sockets: FakeSock[];
  tracks: { stopped: boolean }[];
  asked: unknown[];
  contexts: CtxLike[];
  nodes: (NodeLike & { name: string; opts: any; connected: boolean })[];
  timers: (() => void)[];
  answer: () => Promise<any>;
}

function fakeEnv(over: Partial<Fake> = {}): Fake {
  const env: Fake = {
    sockets: [],
    tracks: [],
    asked: [],
    contexts: [],
    nodes: [],
    timers: [],
    secure: true,
    answer: () => {
      const tracks = [{ stopped: false }, { stopped: false }];
      env.tracks.push(...tracks);
      return Promise.resolve({ getTracks: () => tracks.map((t) => ({ stop: () => (t.stopped = true) })) });
    },
    media: {
      getUserMedia: (c) => {
        env.asked.push(c);
        return env.answer();
      },
    },
    context: () => {
      const ctx: CtxLike & { resumed: number } = {
        state: "suspended",
        currentTime: 0,
        sampleRate: 48000,
        destination: {},
        onstatechange: null,
        resumed: 0,
        audioWorklet: { addModule: () => Promise.resolve() },
        resume() {
          ctx.resumed++;
          ctx.state = "running";
          ctx.onstatechange && ctx.onstatechange();
          return Promise.resolve();
        },
        close: () => Promise.resolve(),
        createMediaStreamSource: () => ({ port: { onmessage: null, postMessage() {} }, connect() {}, disconnect() {} }),
      };
      env.contexts.push(ctx);
      return ctx;
    },
    node: (_ctx, name, opts) => {
      const n = {
        name,
        opts,
        connected: true,
        port: { onmessage: null as any, postMessage() {} },
        connect() {},
        disconnect() {
          n.connected = false;
        },
      };
      env.nodes.push(n);
      return n;
    },
    socket: (url) => {
      const s = new FakeSock(url);
      env.sockets.push(s);
      return s;
    },
    origin: "ws://host",
    worklet: "/_rw/ext/audio-worklet.js",
    later: (f) => env.timers.push(f),
    ...over,
  };
  return env;
}

let doc: MockDoc;
let el: MockEl;
let reports: string[];

function audioEl(attrs: Record<string, string>) {
  const e = doc.createElement("div");
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  const input = doc.createElement("input");
  input.setAttribute("data-audio-report", "1");
  (input as any).dispatchEvent = (ev: Event) => {
    assert.equal(ev.type, "change", "sent at once: `input` is debounced");
    assert.equal(ev.bubbles, true);
    reports.push(input.value);
  };
  e.appendChild(input);
  return e;
}

beforeEach(() => {
  reset();
  doc = makeDom().document;
  reports = [];
  el = audioEl({ "data-audio": "voice", "data-audio-ws": "/_rw/ext/audio/ws", "data-audio-ticket": "rwt_a", "data-audio-capture": "0" });
  doc.body.appendChild(el);
});

const tick = () => new Promise((r) => setTimeout(r, 0));

async function stream(env: Env, capture = "0") {
  el.setAttribute("data-audio-capture", capture);
  const s = new Stream(el as unknown as Element, env);
  s.context();
  s.sync();
  await tick();
  await tick();
  return s;
}

test("capture(true) asks for the microphone with echo cancellation, noise suppression and auto gain", async () => {
  const env = fakeEnv();
  await stream(env, "1");
  assert.deepEqual(env.asked, [CONSTRAINTS]);
  const a = (env.asked[0] as any).audio;
  assert.equal(a.echoCancellation, true);
  assert.equal(a.noiseSuppression, true);
  assert.equal(a.autoGainControl, true);
  assert.ok(reports.includes("capture granted"), reports.join("|"));
  const node = env.nodes.find((n) => n.name === "rw-capture")!;
  assert.equal(node.opts.numberOfOutputs, 0, "pulled without reaching the speakers");
});

test("a worklet frame goes on the socket as 653 bytes, and its level on the element", async () => {
  const env = fakeEnv();
  const s = await stream(env, "1");
  env.sockets[0].open();
  const node = env.nodes.find((n) => n.name === "rw-capture")!;
  const pcm = new Int16Array(FRAME).fill(16384);
  node.port.onmessage!({ data: { seq: 3, t: 0.5, pcm } });
  assert.equal(env.sockets[0].sent.length, 1);
  assert.equal(env.sockets[0].sent[0].byteLength, 653);
  assert.equal(new DataView(env.sockets[0].sent[0]).getUint32(1, true), 3);
  // The level stays in the page: a CSS variable for a meter, never sent.
  const props: Record<string, string> = {};
  (el.style as any).setProperty = (k: string, v: string) => (props[k] = v);
  node.port.onmessage!({ data: { seq: 4, t: 0.52, pcm } });
  assert.equal(props["--rw-level"], "0.500");
  assert.equal(env.sockets[0].sent.length, 2);
  assert.ok(s.ws);
});

test("the four capture states", async () => {
  await stream(fakeEnv({ secure: false }), "1");
  assert.deepEqual(reports.filter((r) => r.startsWith("capture")), ["capture insecure"]);

  reset();
  reports = [];
  await stream(fakeEnv({ media: undefined }), "1");
  assert.deepEqual(reports.filter((r) => r.startsWith("capture")), ["capture unavailable"]);

  for (const [name, want] of [
    ["NotAllowedError", "capture denied"],
    ["SecurityError", "capture denied"],
    ["NotFoundError", "capture unavailable"],
    ["NotReadableError", "capture unavailable"],
  ]) {
    reports = [];
    const e = Object.assign(new Error(name), { name });
    await stream(fakeEnv({ answer: () => Promise.reject(e) }), "1");
    assert.deepEqual(reports.filter((r) => r.startsWith("capture")), [want], name);
  }

  reports = [];
  await stream(fakeEnv(), "1");
  assert.deepEqual(reports.filter((r) => r.startsWith("capture")), ["capture granted"]);
});

test("capture(false) stops every track (the browser's microphone indicator goes off)", async () => {
  const env = fakeEnv();
  const s = await stream(env, "1");
  assert.equal(env.tracks.length, 2);
  assert.ok(env.tracks.every((t) => !t.stopped));
  el.setAttribute("data-audio-capture", "0");
  s.sync();
  assert.ok(env.tracks.every((t) => t.stopped), "track.stop() on each track");
  assert.ok(env.nodes.every((n) => !n.connected));
  // And again: a new capture, new tracks.
  el.setAttribute("data-audio-capture", "1");
  s.sync();
  await tick();
  await tick();
  assert.equal(env.tracks.length, 4);
});

test("capture(false) while the browser asks: the late answer is released at once", async () => {
  let answer!: (m: any) => void;
  const tracks = [{ stopped: false }];
  const env = fakeEnv({
    answer: () => new Promise((r) => (answer = r)),
  });
  const s = await stream(env, "1");
  el.setAttribute("data-audio-capture", "0");
  s.sync();
  answer({ getTracks: () => tracks.map((t) => ({ stop: () => (t.stopped = true) })) });
  await tick();
  assert.ok(tracks[0].stopped);
  assert.ok(!reports.includes("capture granted"));
});

test("the output is blocked until a gesture resumes the context, then ready; interrupted after", async () => {
  const env = fakeEnv();
  const s = await stream(env);
  assert.deepEqual(reports, ["output blocked"]);
  s.gesture();
  await tick();
  assert.deepEqual(reports, ["output blocked", "output ready"]);
  const ctx = env.contexts[0];
  ctx.state = "suspended";
  ctx.onstatechange!();
  assert.deepEqual(reports, ["output blocked", "output ready", "output interrupted"]);
});

test("a gesture anywhere in the page resumes the context, inside the gesture", async () => {
  const env = fakeEnv();
  install(doc as unknown as Document, env);
  const ctx = env.contexts[0] as CtxLike & { resumed: number };
  assert.equal(ctx.state, "suspended");
  assert.ok(current);
  for (const fn of doc.listeners.pointerdown) fn({});
  assert.equal(ctx.resumed, 1);
  assert.equal(ctx.state, "running");
});

test("the socket opens with the rendered ticket, then reconnects with the next one", async () => {
  const env = fakeEnv();
  const s = await stream(env);
  assert.equal(env.sockets.length, 1);
  assert.equal(env.sockets[0].url, "ws://host/_rw/ext/audio/ws?t=rwt_a");
  assert.equal(env.sockets[0].binaryType, "arraybuffer");
  env.sockets[0].open();
  env.sockets[0].serverSends(ticketMsg("rwt_b"));
  assert.equal(s.next, "rwt_b");
  env.sockets[0].drop(1006);
  assert.equal(env.timers.length, 1, "a retry is scheduled");
  env.timers.shift()!();
  assert.equal(env.sockets[1].url, "ws://host/_rw/ext/audio/ws?t=rwt_b");
  // Opened (rwt_b spent), then lost before any ticket: the spent render ticket is not reused.
  env.sockets[1].open();
  env.sockets[1].drop(1006);
  env.timers.shift()!();
  assert.equal(env.sockets.length, 2, "nothing to present: wait for a render");
  // A render brings a new ticket.
  el.setAttribute("data-audio-ticket", "rwt_c");
  s.sync();
  assert.equal(env.sockets[2].url, "ws://host/_rw/ext/audio/ws?t=rwt_c");
});

test("a refused ticket (4403) is not retried", async () => {
  const env = fakeEnv();
  const s = await stream(env);
  env.sockets[0].serverSends(ticketMsg("rwt_b"));
  env.sockets[0].drop(4403);
  assert.equal(env.timers.length, 0);
  assert.equal(s.next, null);
  s.sync();
  assert.equal(env.sockets.length, 1);
});

test("the socket opens only while an audio element is in the page, and closes when it goes", async () => {
  el.parentNode!.removeChild(el);
  const env = fakeEnv();
  install(doc as unknown as Document, env);
  assert.equal(env.sockets.length, 0);
  assert.equal(env.contexts.length, 0);
  reset();
  doc.body.appendChild(el);
  const env2 = fakeEnv();
  const s = new Stream(el as unknown as Element, env2);
  s.sync();
  assert.equal(env2.sockets.length, 1);
  s.destroy();
  assert.equal(env2.sockets[0].closed, 1000);
});

// --- the real worklet, run on fake audio-thread globals ---

test("seq restarts at 0 at each capture start (each capture(true)), counted by the real worklet", async () => {
  const g = globalThis as any;
  const registry: Record<string, any> = {};
  g.sampleRate = 48000;
  g.currentFrame = 0;
  g.AudioWorkletProcessor = class {
    port: any = { postMessage: (m: any) => this.port.deliver && this.port.deliver(m) };
  };
  g.registerProcessor = (name: string, ctor: unknown) => (registry[name] = ctor);
  await import("../src/ext/audio-worklet.ts");
  assert.ok(registry["rw-capture"], "the worklet registers rw-capture");

  const procs: any[] = [];
  const env = fakeEnv({
    node: (_ctx, name, opts) => {
      const proc = new registry[name]();
      procs.push(proc);
      const n: any = {
        name,
        opts,
        connected: true,
        port: { onmessage: null, postMessage() {} },
        connect() {},
        disconnect() {
          n.connected = false;
        },
      };
      proc.port.deliver = (m: any) => n.port.onmessage && n.port.onmessage({ data: m });
      env.nodes.push(n);
      return n;
    },
  });
  /** Run the audio thread for `ms` on the newest processor. */
  const run = (ms: number) => {
    const proc = procs[procs.length - 1];
    for (let k = 0; k < (48 * ms) / 128; k++) {
      proc.process([[sine(48000, 440, 128, g.currentFrame)]]);
      g.currentFrame += 128;
    }
  };
  const seqs = () => env.sockets[0].sent.map((b) => new DataView(b).getUint32(1, true));

  const s = await stream(env, "1");
  env.sockets[0].open();
  run(100);
  assert.deepEqual(seqs(), [0, 1, 2, 3, 4]);

  el.setAttribute("data-audio-capture", "0");
  s.sync();
  el.setAttribute("data-audio-capture", "1");
  s.sync();
  await tick();
  await tick();
  assert.equal(procs.length, 2, "a new capture is a new worklet node");
  run(60);
  assert.deepEqual(seqs(), [0, 1, 2, 3, 4, 0, 1, 2], "back to 0 at the new capture");
  const ts = env.sockets[0].sent.map((b) => new DataView(b).getFloat64(5, true));
  assert.ok(ts[5] > ts[4], "time keeps running on the audio clock");
});

// --- review fixes ---

test("a reconnection that fails before opening keeps the next ticket and tries it again", async () => {
  const env = fakeEnv();
  const s = await stream(env);
  env.sockets[0].open();
  env.sockets[0].serverSends(ticketMsg("rwt_b"));
  env.sockets[0].drop(1006);
  env.timers.shift()!();
  assert.equal(env.sockets[1].url, "ws://host/_rw/ext/audio/ws?t=rwt_b");
  // The server is down: the handshake fails, the ticket was never presented.
  env.sockets[1].drop(1006);
  env.timers.shift()!();
  assert.equal(env.sockets.length, 3, "retried");
  assert.equal(env.sockets[2].url, "ws://host/_rw/ext/audio/ws?t=rwt_b");
  // Once open, the ticket is spent: it is not kept.
  env.sockets[2].open();
  assert.equal(s.next, null);
});

test("a reconnection that cannot go on says so: output interrupted, then the state again once back", async () => {
  const env = fakeEnv();
  const s = await stream(env);
  s.gesture();
  await tick();
  env.sockets[0].open();
  env.sockets[0].serverSends(ticketMsg("rwt_b"));
  env.sockets[0].drop(1006);
  env.timers.shift()!();
  reports = [];
  env.sockets[1].drop(4403);
  assert.deepEqual(reports, ["output interrupted"], "the app can redraw");
  // A render brings a ticket: once the socket is back, the output state is said again.
  el.setAttribute("data-audio-ticket", "rwt_c");
  s.sync();
  env.sockets[2].open();
  assert.deepEqual(reports, ["output interrupted", "output ready"]);
});

test("a jump of the audio clock realigns the capture: the next frame is timed from the new position", () => {
  const c = new Capture(48000);
  c.push(new Float32Array(128), 0);
  // The thread was suspended: the next block starts far later.
  const frames = [];
  for (let at = 96000; at < 96000 + 48000 * 0.1; at += 128) frames.push(...c.push(new Float32Array(128), at));
  assert.ok(frames.length >= 4);
  assert.equal(frames[0].t, 2, "timed at the new position (96000 / 48000), not at 128 samples in");
  assert.ok(Math.abs(frames[1].t - 2.02) < 1e-9);
  assert.equal(frames[1].seq, frames[0].seq + 1, "the numbering goes on");
});
