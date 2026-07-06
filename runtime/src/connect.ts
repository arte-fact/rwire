// WebSocket lifecycle: connect, exponential-backoff reconnect (cap 30s), the
// full client-state reset on reopen (the server replays the page onto a clean
// slate; name maps intentionally survive — MAP_DEF re-delivery is per
// connection), and the stale-capsule escape hatch (after 2 failed retries,
// reload once /ready answers — a deploy may have shipped a new capsule).

import { st, resetSession } from "./state.ts";
import { resetActions } from "./actions.ts";
import { x } from "./executor.ts";
import { ov } from "./overlay.ts";
import { sh } from "./hash.ts";

let rc = 0; // retry count
let rn = false; // reconnecting (a previous socket existed)
let op = false; // a connection has opened at least once
let ot: ReturnType<typeof setTimeout>; // overlay delay timer

/** Strip the BASE mount prefix: browser path → server route. */
export function bx(p: string): string {
  return BASE && p.slice(0, BASE.length) === BASE ? p.slice(BASE.length) || "/" : p;
}

/** Join the BASE mount prefix: server route → browser path. */
export function bj(u: string): string {
  return BASE + u;
}

export function connect(): void {
  const w = new WebSocket(
    (location.protocol === "https:" ? "wss://" : "ws://") + location.host + BASE,
  );
  st.w = w;
  w.binaryType = "arraybuffer";
  w.onopen = () => {
    clearTimeout(ot);
    ov(false);
    if (rn) {
      // Full re-render incoming: clear everything the server owns.
      document.body
        .querySelectorAll(":scope>:not(script):not(style)")
        .forEach((c) => c.remove());
      resetSession();
      // Future local-handler state hook (ls/lh globals, WASM builds).
      const g = globalThis as any;
      if (typeof g.ls !== "undefined") {
        g.ls = {};
        g.lh = {};
      }
      resetActions();
    }
    rn = false;
    rc = 0;
    op = true;
    if (bx(location.pathname) !== "/") w.send("R" + bx(location.pathname));
    if (location.hash) sh(location.hash);
  };
  w.onmessage = (e) => x(new Uint8Array(e.data as ArrayBuffer));
  w.onclose = () => {
    rn = true;
    clearTimeout(ot);
    ot = setTimeout(() => ov(true, !navigator.onLine), 600);
    if (op && rc >= 2)
      fetch("/ready", { cache: "no-store" })
        .then(() => location.reload())
        .catch(() => {});
    setTimeout(connect, Math.min(1000 * Math.pow(2, rc++), 30000));
  };
  w.onerror = () => {};
}

/** Overlay Retry button: reset backoff and reconnect now. */
export function retryConnect(): void {
  rc = 0;
  connect();
}

/** `online` event: reconnect immediately if the socket is dead. */
export function reconnectIfDead(): void {
  if (st.w!.readyState > 1) {
    rc = 0;
    connect();
  }
}

/** `offline` event: show the offline overlay if we were reconnecting. */
export function offlineNotice(): void {
  if (rn) ov(true, true);
}

/** Tab became visible with a dead socket: reconnect without waiting out the backoff. */
export function onVisibilityChange(): void {
  if (!document.hidden && st.w!.readyState > 1) {
    rc = 0;
    connect();
  }
}
