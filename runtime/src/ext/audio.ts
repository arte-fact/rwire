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
//   `<input data-audio-report>` and a bubbling `input` event dispatched,
//   so the handler bound there (`rwire_audio::parse_event`) learns them:
//     "capture granted|denied|unavailable|insecure"
//     "output ready|blocked|interrupted"
// - The audio context is resumed on any user gesture in the page (pointer,
//   key, touch), in that gesture: browsers keep a context made without one
//   suspended.

import { encodeFrame, level, type Frame } from "./audio-dsp.ts";

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
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }

  /** Bring the stream in line with the element's attributes. */
  sync(): void {
    if (!this.alive) return;
    if (!this.ws) this.connect();
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
    if (b[0] === 0x80) this.next = new TextDecoder().decode(b.subarray(1));
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
      attributeFilter: ["data-audio", "data-audio-ticket", "data-audio-capture"],
    });
}

/** Forget the page's stream (tests). */
export function reset(): void {
  if (current) current.destroy();
  current = null;
}
