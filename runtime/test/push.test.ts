// Push extension: the gesture-bound subscribe, the unsubscribe, the first
// status of a report input, and what reaches the server (the report input's
// value + a bubbling `input` event). The browser surface (service worker,
// Notification, PushManager) is a fake Env.
import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { makeDom, type MockDoc, type MockEl } from "./dom.ts";
import { b64u, i as install, subscribe, unsubscribe, status, type Env } from "../src/ext/push.ts";

let doc: MockDoc;
let input: MockEl;
let reports: any[];

const KEY = "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8";

beforeEach(() => {
  doc = makeDom().document;
  input = doc.createElement("input");
  input.setAttribute("data-push-report", "1");
  (input as any).dispatchEvent = (e: Event) => {
    assert.equal(e.type, "input");
    assert.equal(e.bubbles, true, "the delegated input listener must see it");
    reports.push(JSON.parse(input.value));
  };
  doc.body.appendChild(input);
  reports = [];
});

function fakeSub(endpoint: string, key: Uint8Array) {
  const s = {
    endpoint,
    options: { applicationServerKey: key.buffer },
    dropped: false,
    toJSON: () => ({ endpoint, keys: { p256dh: "pk", auth: "au" } }),
    unsubscribe: async () => ((s.dropped = true), true),
  };
  return s;
}

/** A browser whose permission answer and current subscription are chosen. */
function env(opts: { permission?: string; answer?: string; current?: any } = {}) {
  const calls: string[] = [];
  let current = opts.current ?? null;
  const e: Env & { calls: string[] } = {
    calls,
    push: true,
    notif: {
      permission: opts.permission ?? "default",
      requestPermission: () => {
        calls.push("requestPermission");
        return Promise.resolve(opts.answer ?? "granted");
      },
    },
    sw: {
      ready: Promise.resolve({
        pushManager: {
          getSubscription: async () => current,
          subscribe: async (o: any) => {
            calls.push("subscribe:" + o.userVisibleOnly + ":" + o.applicationServerKey.length);
            current = fakeSub("https://push.example/new", o.applicationServerKey);
            return current;
          },
        },
      }),
    },
  };
  return e;
}

test("base64url decodes the VAPID key to its 65 bytes", () => {
  const k = b64u(KEY);
  assert.equal(k.length, 65);
  assert.equal(k[0], 4, "uncompressed point");
});

test("subscribe asks the permission synchronously, inside the gesture", async () => {
  const e = env();
  const p = subscribe(doc as any, KEY, e);
  // Before any await resolves: the prompt was already requested.
  assert.deepEqual(e.calls, ["requestPermission"]);
  await p;
  assert.deepEqual(e.calls, ["requestPermission", "subscribe:true:65"]);
  assert.equal(reports.length, 1);
  assert.equal(reports[0].state, "subscribed");
  assert.equal(reports[0].subscription.endpoint, "https://push.example/new");
  assert.equal(reports[0].permission, "granted");
});

test("an existing subscription under the same key is reused", async () => {
  const cur = fakeSub("https://push.example/old", b64u(KEY));
  const e = env({ permission: "granted", current: cur });
  await subscribe(doc as any, KEY, e);
  assert.deepEqual(e.calls, [], "no prompt once granted, no new subscription");
  assert.equal(reports[0].subscription.endpoint, "https://push.example/old");
});

test("a subscription under another key is replaced", async () => {
  const cur = fakeSub("https://push.example/old", new Uint8Array(65));
  const e = env({ permission: "granted", current: cur });
  await subscribe(doc as any, KEY, e);
  assert.equal(cur.dropped, true);
  assert.equal(reports[0].subscription.endpoint, "https://push.example/new");
});

test("a refused permission reports denied and subscribes nothing", async () => {
  const e = env({ answer: "denied" });
  await subscribe(doc as any, KEY, e);
  assert.deepEqual(e.calls, ["requestPermission"]);
  assert.deepEqual(reports, [{ state: "denied", permission: "denied" }]);
});

test("without PushManager (iOS outside an installed PWA) it reports unsupported", async () => {
  const e = env();
  e.push = false;
  await subscribe(doc as any, KEY, e);
  assert.deepEqual(e.calls, []);
  assert.deepEqual(reports, [{ state: "unsupported" }]);
});

test("a failing subscribe reports the error", async () => {
  const e = env({ permission: "granted" });
  (await e.sw!.ready).pushManager.subscribe = () => Promise.reject(new Error("AbortError"));
  await subscribe(doc as any, KEY, e);
  assert.deepEqual(reports, [{ state: "error", error: "AbortError" }]);
});

test("unsubscribe drops the subscription and names its endpoint", async () => {
  const cur = fakeSub("https://push.example/old", b64u(KEY));
  await unsubscribe(doc as any, env({ permission: "granted", current: cur }));
  assert.equal(cur.dropped, true);
  assert.deepEqual(reports, [
    { state: "unsubscribed", endpoint: "https://push.example/old", permission: "granted" },
  ]);
});

test("status reports without prompting", async () => {
  const e = env();
  await status(doc as any, e);
  assert.deepEqual(e.calls, []);
  assert.deepEqual(reports, [{ state: "unsubscribed", endpoint: null, permission: "default" }]);
});

test("install reports the first status and delegates clicks", async () => {
  const e = env();
  install(doc as any, e);
  await new Promise((r) => setTimeout(r, 0));
  assert.equal(reports.length, 1, "first status of the report input");
  assert.equal(reports[0].state, "unsubscribed");

  const btn = doc.createElement("button");
  btn.setAttribute("data-push", "subscribe");
  btn.setAttribute("data-push-key", KEY);
  btn.closest = () => btn;
  doc.body.appendChild(btn);
  btn.click();
  assert.deepEqual(e.calls, ["requestPermission"], "prompt inside the click");
  await new Promise((r) => setTimeout(r, 0));
  assert.equal(reports.at(-1).state, "subscribed");

  // A click elsewhere does nothing.
  const other = doc.createElement("div");
  other.closest = () => null;
  doc.body.appendChild(other);
  other.click();
  assert.equal(e.calls.length, 2);
});

test("no report input: the outcome is dropped, nothing throws", async () => {
  doc.body.removeChild(input);
  await subscribe(doc as any, KEY, env());
  assert.deepEqual(reports, []);
});

test("ext/push.min.js stays within its own budget", async () => {
  const { readFileSync } = await import("node:fs");
  const out = readFileSync(new URL("../dist/ext/push.min.js", import.meta.url));
  assert.ok(out.length <= 3_000, `push ext ${out.length} bytes exceeds 3000`);
});
