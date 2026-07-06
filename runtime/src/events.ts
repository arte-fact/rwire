// Event → server messages. Wire format (decoded by protocol/decoder.rs):
//   plain:  [0x00, handler_varint, event_type, ref&255, payload_len_varint, payload…]
//   param:  [0x80, handler_varint, event_type, ref&255, param_len, params…,
//            payload_len_varint, payload…]

import { st, type RwEl } from "./state.ts";
import { wv } from "./varint.ts";

/** Collect the event payload: form values on submit, control value on
 * input/change, data-* attrs on click. Empty string when nothing applies. */
export function gp(e: Event, el: RwEl): string {
  const t = el.tagName.toLowerCase();
  if (e.type === "submit" && t === "form") {
    e.preventDefault();
    const fd = new FormData(el as unknown as HTMLFormElement),
      obj: Record<string, unknown> = {};
    fd.forEach((v, k) => (obj[k] = v));
    return JSON.stringify({ t: "form", v: obj });
  }
  if (
    (e.type === "input" || e.type === "change") &&
    (t === "input" || t === "textarea" || t === "select")
  ) {
    return JSON.stringify({ t: "text", v: (el as HTMLInputElement).value });
  }
  if (e.type === "click") {
    const tg = ((e.target as Element).closest("[data-id]") || el) as HTMLElement,
      dt: Record<string, string> = {};
    for (const k in tg.dataset) dt[k] = tg.dataset[k]!;
    if (Object.keys(dt).length) return JSON.stringify({ t: "data", v: dt });
  }
  return "";
}

/** Send a plain remote event. */
export function se(h: number, t: number, f: number, e: Event, el: RwEl): void {
  const p = gp(e, el),
    pb = new TextEncoder().encode(p),
    a: number[] = [0];
  wv(a, h);
  a.push(t, f & 255);
  wv(a, pb.length);
  const msg = new Uint8Array(a.length + pb.length);
  for (let j = 0; j < a.length; j++) msg[j] = a[j];
  msg.set(pb, a.length);
  st.w!.send(msg);
}

/** Send a remote event carrying ItemRef param bytes. */
export function sep(
  h: number,
  t: number,
  f: number,
  prm: Uint8Array,
  e: Event,
  el: RwEl,
): void {
  const p = gp(e, el),
    pb = new TextEncoder().encode(p),
    a: number[] = [0x80];
  wv(a, h);
  a.push(t, f & 255, prm.length);
  const pl: number[] = [];
  wv(pl, pb.length);
  const msg = new Uint8Array(a.length + prm.length + pl.length + pb.length);
  let j = 0;
  for (const b of a) msg[j++] = b;
  msg.set(prm, j);
  j += prm.length;
  for (const b of pl) msg[j++] = b;
  msg.set(pb, j);
  st.w!.send(msg);
}

/** Debounce input events 250ms per element (`__t` timer expando); everything
 * else fires immediately. */
export function snd(fn: () => void, e: Event, el: RwEl): void {
  if (e.type === "input") {
    if (el.__t) clearTimeout(el.__t);
    el.__t = setTimeout(fn, 250);
  } else fn();
}
