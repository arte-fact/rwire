// Client-side live value bindings (LIVE_SOURCE / LIVE_BIND): an input pushes
// its value on every `input` event to the elements bound to its channel — a
// slider's readout, a fill, a lookup curve, a sum of several sliders — with no
// server round-trip. A binding lives on the bound node (`__lb`) and a source
// registers itself on its node (`__lc`), so the morph carries both across a
// re-render (morph.ts); BATCH_END forgets what left the document and re-runs
// every channel (executor.ts). Kinds and arguments: protocol/opcodes.rs
// `LIVE_BIND`.

import { type RwEl } from "./state.ts";

/** One term of a sum: the channel it reads, its table's offset and entries. */
export interface LiveTerm {
  c: number;
  o: number;
  t: number[];
}

/** A sum: a base plus lookup tables read at their own channels' positions. */
export interface LiveSum {
  b: number;
  s: LiveTerm[];
}

/** A binding as parsed off the wire; `e` is null once its node was morphed away. */
export interface LiveBind {
  e: LiveEl | null;
  k: number;
  /** Remainder: the base and every channel summed (the first included). */
  b?: number;
  c?: number[];
  /** Scaled: numerator, denominator. */
  n?: number;
  d?: number;
  /** Decimal: fraction digits. */
  p?: number;
  /** Switch thresholds, or a lookup table with its offset `o`. */
  t?: number[];
  o?: number;
  /** Sum / span: the low and (with LIVE_RANGE) high table sets. */
  l?: LiveSum;
  h?: LiveSum;
  /** Span: axis and window bounds. */
  a0?: number;
  a1?: number;
  w0?: number;
  w1?: number;
}

/** A node that is a live source (`__lc`, `__ls`) and/or carries bindings (`__lb`). */
export type LiveEl = RwEl & {
  __lc?: number;
  __lb?: LiveBind[];
  __ls?: 1;
};

/** Source element by channel. */
export const lv: Record<number, LiveEl> = {};
/** Bindings by channel. */
export const lb: Record<number, LiveBind[]> = {};

/** Forget every source and binding (a reconnect replays the page). */
export function resetLive(): void {
  for (const c in lv) delete lv[c];
  for (const c in lb) delete lb[c];
}

const lang = () => document.documentElement.lang || undefined;

/** Register `e` as the source of channel `c`; its input events update the channel. */
export function ls(e: LiveEl, c: number): void {
  e.__lc = c;
  lv[c] = e;
  if (!e.__ls) {
    e.__ls = 1;
    e.addEventListener("input", () => {
      if (e.__lc !== undefined) {
        lv[e.__lc] = e;
        ul(e.__lc);
      }
    });
  }
}

/** `t` read at position `p` (0..1), linearly interpolated between its evenly spaced entries. */
export function ip(t: number[], p: number): number {
  const q = Math.min(1, Math.max(0, p)) * (t.length - 1),
    j = Math.floor(q);
  return t[j] + (t[Math.min(j + 1, t.length - 1)] - t[j]) * (q - j);
}

/** The value and position (between `min` and `max`) of a source. */
function at(x: LiveEl): [number, number] {
  const s = x as unknown as HTMLInputElement;
  const v = +s.value,
    mn = +s.min || 0,
    mx = +s.max;
  return [v, mx > mn ? (v - mn) / (mx - mn) : 0];
}

/** Update every element bound to channel `c` from its source. */
export function ul(c: number): void {
  const s = lv[c];
  if (!s) return;
  for (const b of lb[c] || []) {
    const e = b.e,
      k = b.k & 15;
    let v: number, p: number;
    if (!e) continue;
    if (b.k & 16) {
      v = b.b! - b.c!.reduce((a, h) => a + (lv[h] ? at(lv[h])[0] || 0 : 0), 0);
      p = b.b! > 0 ? (b.b! - v) / b.b! : 0;
    } else [v, p] = at(s);
    const f = b.k & 32 ? (n: number) => n.toLocaleString(lang()) : String;
    const sg = (y: number) =>
      b.k & 64 ? (y > 0 ? "+" : y < 0 ? "−" : "") + f(Math.abs(y)) : f(y);
    if (k === 1) e.style.width = Math.min(100, Math.max(0, p * 100)) + "%";
    else if (k === 2) e.textContent = f(b.d ? Math.round((v * b.n!) / b.d) : 0);
    else if (k === 3) {
      let i = 0;
      while (i < b.t!.length && v >= b.t![i]) i++;
      for (let j = 0; j < e.children.length; j++)
        (e.children[j] as HTMLElement).hidden = j !== i;
    } else if (k === 4) e.textContent = sg(Math.round(b.o! + ip(b.t!, p)));
    else if (k === 7)
      e.textContent = (v / 10 ** b.p!).toLocaleString(lang(), {
        minimumFractionDigits: b.p,
        maximumFractionDigits: b.p,
      });
    else if (k === 5 || k === 6) {
      const g = (a: LiveSum) =>
        Math.round(
          a.b +
            a.s.reduce((z, t) => {
              const x = lv[t.c];
              return x ? z + t.o + ip(t.t, at(x)[1]) : z;
            }, 0),
        );
      let y = g(b.l!),
        z = b.h ? g(b.h) : y;
      if (k === 6) {
        const cl = (n: number) => Math.min(b.w1!, Math.max(b.w0!, n)),
          w = b.a1! - b.a0! || 1;
        y = cl(y);
        z = cl(z);
        e.style.left = ((y - b.a0!) / w) * 100 + "%";
        e.style.width = (Math.max(0, z - y) / w) * 100 + "%";
      } else e.textContent = y === z ? sg(y) : sg(y) + " … " + sg(z);
    } else e.textContent = f(v);
  }
}
