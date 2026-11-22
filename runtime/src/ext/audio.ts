// Audio extension: microphone capture and playback over the page's audio
// socket (rwire-audio).
//
// Lazy-loaded runtime extension: NOT part of the core capsule. The DOM
// contract (rendered by `rwire_audio::AudioExt::audio`):
// - `[data-audio="<name>"]`, one per page, carries the socket's path
//   (`data-audio-ws`) and a single-use ticket (`data-audio-ticket`). The
//   socket opens while such an element is in the page, and closes when it
//   goes. Each connection receives the ticket of the next one (0x80), so a
//   lost socket reconnects by itself; a refused ticket (close 4403) waits
//   for a render to bring another.
// - `data-audio-capture="1"` asks for the microphone (echo cancellation,
//   noise suppression, automatic gain); "0" stops every track. Frames of
//   20 ms, 16 kHz mono PCM16, are made by the `rw-capture` worklet and sent
//   with a sequence number and their audio-clock time.
// - The level of the last frame (0..1) is set as `--rw-level` on the
//   element, for a meter. It never leaves the browser.
// - Outcomes are written as a line into the value of the element's
//   `<input data-audio-report>` and a bubbling `change` event dispatched,
//   so the handler bound there (`rwire_audio::parse_event`) learns them:
//     "capture granted|denied|unavailable|insecure"
//     "output ready|blocked|interrupted"
//     "started <t>" | "mark <t> <id>" | "underrun <t>" | "ended <t>"
//     "stopped <t> <samples since the last mark> [<last mark>]"
//   `<t>` is in seconds on the page's audio clock (the clock of the
//   microphone's frames), when the sample is heard: the context's output
//   latency is added, and the line is written at that time.
// - Playback, from the socket: PCM (resampled from its rate to the
//   context's), marks, preloaded sounds (`data-audio-sounds`, JSON
//   `[[name, url], …]`) and the end of the utterance are queued in arrival
//   order and played gaplessly by the `rw-play` worklet; "stop" drops the
//   queue at the next render quantum.
// - The audio context is resumed on any user gesture in the page (pointer,
//   key, touch), in that gesture: browsers keep a context made without one
//   suspended.

import { encodeFrame, level, type Frame } from "./audio-dsp.ts";
import { Resampler, type Item, type Note } from "./audio-play.ts";

/** Version stamp logged by the loader — bump with every change. */
export const v = 1;

/** The close code of a refused ticket (rwire::ext_socket). */
export const CLOSE_REFUSED = 4403;
/** Reconnection delays, ms: doubled from the first to the last. */
const RETRY_MIN = 250;
const RETRY_MAX = 5000;

export const CONSTRAINTS = {
  audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true, channelCount: 1 },
};

export interface TrackLike {
  stop(): void;
}
export interface MediaLike {
  getTracks(): TrackLike[];
}
export interface PortLike {
  onmessage: ((m: { data: any }) => void) | null;
  postMessage(m: unknown, transfer?: unknown[]): void;
}
export interface NodeLike {
  port: PortLike;
  connect(n: unknown): void;
  disconnect(): void;
}
export interface BufferLike {
  getChannelData(c: number): Float32Array;
}
export interface CtxLike {
  state: string;
  currentTime: number;
  sampleRate: number;
  outputLatency?: number;
  baseLatency?: number;
  destination: unknown;
  onstatechange: (() => void) | null;
  audioWorklet: { addModule(url: string): Promise<void> };
  resume(): Promise<void>;
  close(): Promise<void>;
  createMediaStreamSource(m: MediaLike): NodeLike;
  decodeAudioData(b: ArrayBuffer): Promise<BufferLike>;
}
export interface SockLike {
  binaryType: string;
  readyState: number;
  onopen: (() => void) | null;
  onmessage: ((m: { data: ArrayBuffer | string }) => void) | null;
  onclose: ((e: { code: number }) => void) | null;
  send(d: ArrayBuffer): void;
  close(code?: number): void;
}

/** The browser surface the extension needs; injectable for tests. */
export interface Env {
  /** `isSecureContext`. */
  secure: boolean;
  /** `navigator.mediaDevices`, if any. */
  media?: { getUserMedia(c: typeof CONSTRAINTS): Promise<MediaLike> };
  /** A new `AudioContext`, if Web Audio exists. */
  context?: () => CtxLike;
  /** A new `AudioWorkletNode`. */
  node(ctx: CtxLike, name: string, opts: object): NodeLike;
  /** A new `WebSocket` to `url`. */
  socket(url: string): SockLike;
  /** `ws[s]://host` + the mount prefix. */
  origin: string;
  /** Where the worklet module is served. */
  worklet: string;
  later(f: () => void, ms: number): void;
  /** `fetch`, for the preloaded sounds. */
  fetch(url: string): Promise<{ arrayBuffer(): Promise<ArrayBuffer> }>;
}

const browserEnv = (): Env => {
  const g = globalThis as any;
  const base = typeof g.BASE === "string" ? g.BASE : "";
  const AC = g.AudioContext || g.webkitAudioContext;
  return {
    secure: !!g.isSecureContext,
    media: g.navigator && g.navigator.mediaDevices && g.navigator.mediaDevices.getUserMedia ? g.navigator.mediaDevices : undefined,
    context: AC && g.AudioWorkletNode ? () => new AC() : undefined,
    node: (ctx, name, opts) => new g.AudioWorkletNode(ctx, name, opts),
    socket: (url) => new g.WebSocket(url),
    origin: (g.location.protocol === "https:" ? "wss://" : "ws://") + g.location.host + base,
    worklet: base + "/_rw/ext/audio-worklet.js",
    later: (f, ms) => g.setTimeout(f, ms),
    fetch: (url) => g.fetch(base + url, { credentials: "same-origin" }),
  };
};

/** The capture state a `getUserMedia` failure means. */
export const refusal = (e: any): string =>
  e && (e.name === "NotAllowedError" || e.name === "SecurityError" || e.name === "PermissionDeniedError")
    ? "denied"
    : "unavailable";

/** One page's audio stream: its socket, its context, its microphone. */
export class Stream {
  ws: SockLike | null = null;
  /** The ticket of the next connection, if one was handed over. */
  next: string | null = null;
  /** Render tickets already presented (a ticket is single-use). */
  private spent = new Set<string>();
  private retry = RETRY_MIN;
  private alive = true;
  ctx: CtxLike | null = null;
  private ready: Promise<CtxLike | null> | null = null;
  private wasRunning = false;
  private output = "";
  /** Whether the render asks for the microphone. */
  private want = false;
  /** Bumped at each capture start and stop: a late `getUserMedia` answer of an old one is dropped. */
  private gen = 0;
  media: MediaLike | null = null;
  private source: NodeLike | null = null;
  private capNode: NodeLike | null = null;
  /** The `rw-play` node, once the context is ready. */
  private play: Promise<NodeLike | null> | null = null;
  /** Playback operations run in arrival order: a sound still loading holds what follows. */
  private chain: Promise<unknown> = Promise.resolve();
  /** Bumped by stop: operations queued before it are dropped. */
  private playGen = 0;
  private resamplers = new Map<number, Resampler>();
  private sounds = new Map<string, Promise<Float32Array | null>>();

  readonly el: Element;
  readonly env: Env;

  constructor(el: Element, env: Env) {
    this.el = el;
    this.env = env;
  }

  /** Write `line` into the report input and tell the server. */
  report(line: string): void {
    const input = this.el.querySelector("[data-audio-report]") as HTMLInputElement | null;
    if (!input) return;
    input.value = line;
    // `change`, not `input`: the runtime debounces `input` (250 ms, last
    // value wins), which would merge lines written in one burst (a mark and
    // the end); `change` is sent at once, each with its own line.
    input.dispatchEvent(new Event("change", { bubbles: true }));
  }

  /** Bring the stream in line with the element's attributes. */
  sync(): void {
    if (!this.alive) return;
    if (!this.ws) this.connect();
    this.loadSounds();
    const want = this.el.getAttribute("data-audio-capture") === "1";
    if (want !== this.want) {
      this.want = want;
      if (want) this.startCapture();
      else this.stopCapture();
    }
  }

  // --- socket ---

  connect(): void {
    const rendered = this.el.getAttribute("data-audio-ticket");
    let t = this.next;
    if (!t && rendered && !this.spent.has(rendered)) t = rendered;
    if (!t || !this.alive) return;
    if (t === rendered) this.spent.add(t);
    this.next = null;
    const path = this.el.getAttribute("data-audio-ws") || "/_rw/ext/audio/ws";
    const ws = this.env.socket(this.env.origin + path + "?t=" + encodeURIComponent(t));
    ws.binaryType = "arraybuffer";
    this.ws = ws;
    ws.onopen = () => {
      this.retry = RETRY_MIN;
    };
    ws.onmessage = (m) => {
      if (typeof m.data !== "string") this.message(new Uint8Array(m.data));
    };
    ws.onclose = (e) => {
      if (this.ws !== ws) return;
      this.ws = null;
      if (!this.alive) return;
      if (e.code === CLOSE_REFUSED) {
        // Refused: no retry with what we hold; a render brings a new ticket.
        this.next = null;
        return;
      }
      const wait = this.retry;
      this.retry = Math.min(RETRY_MAX, this.retry * 2);
      this.env.later(() => {
        if (this.alive && !this.ws) this.connect();
      }, wait);
    };
  }

  /** A message from the server. */
  message(b: Uint8Array): void {
    const text = () => new TextDecoder().decode(b.subarray(1));
    switch (b[0]) {
      case 0x80:
        this.next = text();
        return;
      case 0x81: {
        if (b.length < 5) return;
        const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
        const rate = v.getUint32(1, true);
        const pcm = new Int16Array((b.length - 5) >> 1);
        for (let k = 0; k < pcm.length; k++) pcm[k] = v.getInt16(5 + 2 * k, true);
        return this.enqueue((node, ctx) => {
          let r = this.resamplers.get(rate);
          if (!r) this.resamplers.set(rate, (r = new Resampler(rate, ctx.sampleRate)));
          const data = r.push(pcm);
          node.port.postMessage({ k: "pcm", data, ratio: rate / ctx.sampleRate } as Item, [data.buffer]);
        });
      }
      case 0x82: {
        const id = text();
        return this.enqueue((node) => node.port.postMessage({ k: "mark", id } as Item));
      }
      case 0x83: {
        const name = text();
        const sound = this.sounds.get(name);
        return this.enqueue((node) => {
          if (!sound) return;
          return sound.then((data) => {
            // Sounds are not counted in "samples since the last mark".
            if (data) node.port.postMessage({ k: "pcm", data: data.slice(), ratio: 0 } as Item);
          });
        });
      }
      case 0x84:
        return this.stopPlayback();
      case 0x85:
        this.resamplers.clear();
        return this.enqueue((node) => node.port.postMessage({ k: "end" } as Item));
    }
  }

  /** Run `op` on the play node after every earlier operation, unless a stop comes first. */
  private enqueue(op: (node: NodeLike, ctx: CtxLike) => void | Promise<void>): void {
    const gen = this.playGen;
    this.chain = this.chain
      .then(() => this.player())
      .then((node) => {
        if (node && this.ctx && gen === this.playGen && this.alive) return op(node, this.ctx);
      })
      .catch(() => {});
  }

  /** Stop now: the queue in the worklet is dropped at its next render quantum, and what waits here too. */
  stopPlayback(): void {
    this.playGen++;
    this.resamplers.clear();
    this.chain = Promise.resolve();
    if (this.play) this.play.then((node) => node && node.port.postMessage({ k: "stop" }));
  }

  /** The `rw-play` node, made once the context is ready. */
  player(): Promise<NodeLike | null> {
    if (this.play) return this.play;
    this.play = this.context().then((ctx) => {
      if (!ctx || !this.alive) return null;
      const node = this.env.node(ctx, "rw-play", { numberOfInputs: 0, numberOfOutputs: 1, outputChannelCount: [1] });
      node.port.onmessage = (m) => this.note(m.data as Note);
      node.connect(ctx.destination);
      return node;
    });
    return this.play;
  }

  /** A note of the worklet: reported when its sample is heard, on the audio clock. */
  note(n: Note): void {
    const ctx = this.ctx;
    if (!ctx) return;
    const latency = (ctx.outputLatency || 0) + (ctx.baseLatency || 0);
    const t = n.at / ctx.sampleRate + latency;
    const at = t.toFixed(6);
    let line: string;
    if (n.e === "mark") line = "mark " + at + " " + n.id;
    else if (n.e === "stopped") line = "stopped " + at + " " + n.since + (n.mark ? " " + n.mark : "");
    else line = n.e + " " + at;
    const wait = Math.max(0, (t - ctx.currentTime) * 1000);
    this.env.later(() => this.alive && this.report(line), wait);
  }

  /** Load the sounds the element lists, once each. */
  loadSounds(): void {
    let list: [string, string][] = [];
    try {
      list = JSON.parse(this.el.getAttribute("data-audio-sounds") || "[]");
    } catch {
      return;
    }
    for (const [name, url] of list) {
      if (this.sounds.has(name)) continue;
      this.sounds.set(
        name,
        this.context()
          .then((ctx) => (ctx ? this.env.fetch(url).then((r) => r.arrayBuffer()).then((b) => ctx.decodeAudioData(b)) : null))
          .then((buf) => (buf ? buf.getChannelData(0) : null))
          .catch(() => null),
      );
    }
  }

  /** A frame from the worklet: its level on the element, its bytes on the socket. */
  frame(f: Frame): void {
    const style = (this.el as HTMLElement).style;
    if (style && style.setProperty) style.setProperty("--rw-level", level(f.pcm).toFixed(3));
    if (this.ws && this.ws.readyState === 1) this.ws.send(encodeFrame(f));
  }

  // --- context and output state ---

  /** The audio context, made (and its worklet loaded) once. */
  context(): Promise<CtxLike | null> {
    if (this.ready) return this.ready;
    const make = this.env.context;
    if (!make) return (this.ready = Promise.resolve(null));
    const ctx = make();
    this.ctx = ctx;
    ctx.onstatechange = () => this.outputState();
    this.outputState();
    this.ready = ctx.audioWorklet.addModule(this.env.worklet).then(
      () => ctx,
      () => null,
    );
    return this.ready;
  }

  /** Report the output state when it changes. */
  outputState(): void {
    const ctx = this.ctx;
    if (!ctx || !this.alive) return;
    let s: string;
    if (ctx.state === "running") {
      this.wasRunning = true;
      s = "ready";
    } else if (ctx.state === "closed") return;
    else s = this.wasRunning ? "interrupted" : "blocked";
    if (s !== this.output) {
      this.output = s;
      this.report("output " + s);
    }
  }

  /** In a user gesture: resume the context. */
  gesture(): void {
    if (this.ctx && this.ctx.state !== "running") {
      this.ctx.resume().then(
        () => this.outputState(),
        () => this.outputState(),
      );
    }
  }

  // --- capture ---

  async startCapture(): Promise<void> {
    const gen = ++this.gen;
    if (!this.env.secure) return this.report("capture insecure");
    if (!this.env.media || !this.env.context) return this.report("capture unavailable");
    let media: MediaLike;
    try {
      media = await this.env.media.getUserMedia(CONSTRAINTS);
    } catch (e) {
      if (gen === this.gen) this.report("capture " + refusal(e));
      return;
    }
    if (gen !== this.gen || !this.alive) {
      // Stopped (or gone) while the browser asked: release at once.
      for (const t of media.getTracks()) t.stop();
      return;
    }
    this.media = media;
    this.report("capture granted");
    const ctx = await this.context();
    if (gen !== this.gen || !this.alive) return;
    if (!ctx) {
      this.releaseCapture();
      return this.report("capture unavailable");
    }
    this.gesture();
    const node = this.env.node(ctx, "rw-capture", { numberOfInputs: 1, numberOfOutputs: 0, channelCount: 1 });
    node.port.onmessage = (m) => this.frame(m.data as Frame);
    const source = ctx.createMediaStreamSource(media);
    source.connect(node);
    this.source = source;
    this.capNode = node;
  }

  /** Stop the microphone: every track stopped (the browser's indicator goes off). */
  stopCapture(): void {
    this.gen++;
    this.releaseCapture();
  }

  private releaseCapture(): void {
    if (this.media) for (const t of this.media.getTracks()) t.stop();
    this.media = null;
    if (this.source) this.source.disconnect();
    if (this.capNode) {
      this.capNode.port.onmessage = null;
      this.capNode.disconnect();
    }
    this.source = null;
    this.capNode = null;
  }

  /** The element is gone: everything stops. */
  destroy(): void {
    this.alive = false;
    this.stopCapture();
    const ws = this.ws;
    this.ws = null;
    if (ws) ws.close(1000);
    if (this.ctx) this.ctx.close().catch(() => {});
    this.ctx = null;
  }
}

const installed = new WeakSet<Document>();

/** The stream of the page, if any (one per page). */
export let current: Stream | null = null;

/** Install on a document: follow the `[data-audio]` element and its attributes, and resume audio on gestures. */
export function i(doc: Document, env: Env = browserEnv()): void {
  if (installed.has(doc)) return;
  installed.add(doc);
  const scan = () => {
    const el = doc.querySelector("[data-audio]");
    if (current && current.el !== el) {
      current.destroy();
      current = null;
    }
    if (el && !current) {
      current = new Stream(el, env);
      current.context();
    }
    if (current) current.sync();
  };
  for (const t of ["pointerdown", "keydown", "touchend"])
    doc.addEventListener(t, () => current && current.gesture(), true);
  scan();
  const MO = (globalThis as any).MutationObserver;
  if (MO)
    new MO(scan).observe(doc.documentElement, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ["data-audio", "data-audio-ticket", "data-audio-capture", "data-audio-sounds"],
    });
}

/** Forget the page's stream (tests). */
export function reset(): void {
  if (current) current.destroy();
  current = null;
}
