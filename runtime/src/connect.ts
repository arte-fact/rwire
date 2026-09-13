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
import { resetLive } from "./live.ts";

let rc = 0; // retry count
let rn = false; // reconnecting (a previous socket existed)
let op = false; // a connection has opened at least once
let ot: ReturnType<typeof setTimeout>; // overlay delay timer

// The backoff is reset only when a batch is applied end to end (BATCH_END),
// never on open: a connection that opens, receives the page and dies on a
// parse error escalates to the reload below instead of looping forever.
st.onBatch = () => {
  rc = 0;
};

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
      resetLive();
      resetActions();
    }
    rn = false;
    op = true;
    // The path is reported on every open, `/` included: a returning session
    // whose state still points at a previous page lands on the URL it loaded.
    w.send("R" + bx(location.pathname));
    if (location.hash) sh(location.hash);
  };
  let first = true;
  w.onmessage = (e) => {
    if (first) {
      first = false;
      // Drop the static first paint (SSR) — the live render replaces it.
      document.getElementById("rw")?.remove();
    }
    x(new Uint8Array(e.data as ArrayBuffer));
  };
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
