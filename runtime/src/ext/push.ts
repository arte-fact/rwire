// Push extension: Web Push subscription from a user gesture.
//
// Lazy-loaded runtime extension: NOT part of the core capsule. The DOM
// contract:
// - a click on `[data-push="subscribe"]` (its `data-push-key` is the
//   server's VAPID public key, base64url) asks for the notification
//   permission *inside the click*, the user gesture iOS (16.4+, installed
//   PWA) and Safari require, then subscribes through the service worker's
//   `PushManager` (an existing subscription under another key is replaced);
// - a click on `[data-push="unsubscribe"]` drops the subscription;
// - every outcome is written as JSON into the value of the page's
//   `<input data-push-report>` and a bubbling `input` event dispatched on
//   it, so the handler the server bound there learns it over the
//   WebSocket, behind whatever gate guards the socket:
//     {"state":"subscribed","subscription":{endpoint,keys:{p256dh,auth}},"permission":…}
//     {"state":"unsubscribed","endpoint":…|null,"permission":…}
//     {"state":"unsupported"} | {"state":"denied"} | {"state":"error","error":…}
// - a report input seen for the first time gets the current state, so the
//   page can show "subscribed" from the start.

/** Version stamp logged by the loader — bump with every change. */
export const v = 1;

type Report = Record<string, unknown>;

/** The browser surface the extension needs; injectable for tests. */
export interface Env {
  /** `navigator.serviceWorker`, if any. */
  sw?: { ready: Promise<{ pushManager: PushMgr }> };
  /** `Notification`, if any (absent on iOS outside an installed PWA). */
  notif?: { permission: string; requestPermission(): Promise<string> };
  /** Whether `PushManager` exists. */
  push: boolean;
}
interface Sub {
  endpoint: string;
  options?: { applicationServerKey?: ArrayBuffer | null };
  toJSON(): unknown;
  unsubscribe(): Promise<boolean>;
}
interface PushMgr {
  getSubscription(): Promise<Sub | null>;
  subscribe(o: { userVisibleOnly: boolean; applicationServerKey: Uint8Array }): Promise<Sub>;
}

const browserEnv = (): Env => {
  const g = globalThis as any;
  return {
    sw: g.navigator && g.navigator.serviceWorker,
    notif: g.Notification,
    push: "PushManager" in g,
  };
};

/** base64url → bytes (the VAPID key as the server prints it). */
export function b64u(s: string): Uint8Array {
  const b = atob(s.replace(/-/g, "+").replace(/_/g, "/") + "===".slice((s.length + 3) % 4));
  const out = new Uint8Array(b.length);
  for (let i = 0; i < b.length; i++) out[i] = b.charCodeAt(i);
  return out;
}

const sameKey = (a: ArrayBuffer | null | undefined, b: Uint8Array): boolean => {
  if (!a) return false;
  const x = new Uint8Array(a);
  return x.length === b.length && x.every((v, i) => v === b[i]);
};

/** Write a report into the page's report input and tell the server. */
export function report(doc: Document, r: Report): void {
  const input = doc.querySelector("[data-push-report]") as HTMLInputElement | null;
  if (!input) return;
  input.value = JSON.stringify(r);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

const perm = (env: Env): string => (env.notif ? env.notif.permission : "default");
const supported = (env: Env): boolean => !!(env.sw && env.notif && env.push);

/** Subscribe with `key` (base64url). The permission request is the first
 * thing done, synchronously within the click that called this. */
export function subscribe(doc: Document, key: string, env: Env = browserEnv()): Promise<void> {
  if (!supported(env)) return Promise.resolve(report(doc, { state: "unsupported" }));
  const asked = env.notif!.permission === "granted" ? Promise.resolve("granted") : env.notif!.requestPermission();
  return asked
    .then(async (p) => {
      if (p !== "granted") return report(doc, { state: "denied", permission: p });
      const reg = await env.sw!.ready;
      const k = b64u(key);
      let sub = await reg.pushManager.getSubscription();
      if (sub && !sameKey(sub.options && sub.options.applicationServerKey, k)) {
        // Subscribed under another server key (rotated): start afresh.
        await sub.unsubscribe();
        sub = null;
      }
      if (!sub) sub = await reg.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: k });
      report(doc, { state: "subscribed", subscription: sub.toJSON(), permission: p });
    })
    .catch((e) => report(doc, { state: "error", error: String((e && e.message) || e) }));
}

/** Drop the subscription, if any. */
export function unsubscribe(doc: Document, env: Env = browserEnv()): Promise<void> {
  if (!supported(env)) return Promise.resolve(report(doc, { state: "unsupported" }));
  return env
    .sw!.ready.then(async (reg) => {
      const sub = await reg.pushManager.getSubscription();
      const endpoint = sub ? sub.endpoint : null;
      if (sub) await sub.unsubscribe();
      report(doc, { state: "unsubscribed", endpoint, permission: perm(env) });
    })
    .catch((e) => report(doc, { state: "error", error: String((e && e.message) || e) }));
}

/** The current state, as a report (no prompt, no gesture needed). */
export function status(doc: Document, env: Env = browserEnv()): Promise<void> {
  if (!supported(env)) return Promise.resolve(report(doc, { state: "unsupported" }));
  if (perm(env) === "denied") return Promise.resolve(report(doc, { state: "denied", permission: "denied" }));
  return env
    .sw!.ready.then((reg) => reg.pushManager.getSubscription())
    .then((sub) =>
      report(
        doc,
        sub
          ? { state: "subscribed", subscription: sub.toJSON(), permission: perm(env) }
          : { state: "unsubscribed", endpoint: null, permission: perm(env) },
      ),
    )
    .catch((e) => report(doc, { state: "error", error: String((e && e.message) || e) }));
}

const installed = new WeakSet<Document>();
const seen = new WeakSet<Element>();

/** Install on a document: the click delegation, and the first status of
 * every report input as it appears. */
export function i(doc: Document, env: Env = browserEnv()): void {
  if (installed.has(doc)) return;
  installed.add(doc);
  doc.addEventListener(
    "click",
    (e) => {
      const t = e.target as Element | null;
      const b = t && t.closest ? t.closest("[data-push]") : null;
      if (!b) return;
      const op = b.getAttribute("data-push");
      if (op === "subscribe") subscribe(doc, b.getAttribute("data-push-key") || "", env);
      else if (op === "unsubscribe") unsubscribe(doc, env);
    },
    true,
  );
  const scan = () => {
    const input = doc.querySelector("[data-push-report]");
    if (input && !seen.has(input)) {
      seen.add(input);
      status(doc, env);
    }
  };
  scan();
  const MO = (globalThis as any).MutationObserver;
  if (MO) new MO(scan).observe(doc.documentElement, { childList: true, subtree: true });
}
