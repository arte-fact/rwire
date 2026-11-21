// Full-stack E2E (manual) for Web Push subscription: the shipped runtime
// artifact + the vendored push extension + a real WebSocket against a LIVE
// `examples/push` server. Start it first from a scratch directory (it
// writes push-data/ there): `cargo run -p push`, then
// `node e2e/push.mjs <that dir>/push-data/subscriptions.json` from runtime/.
//
// The browser's Push API is a fake (no push service in a test): it hands
// out a subscription with real P-256 keys. Everything else is real: the
// extension's click delegation, its report through the hidden input, the
// server handler, the stored file, and the re-render of the status.
import { readFileSync } from "node:fs";
import { createECDH, randomBytes } from "node:crypto";
import { makeDom } from "../test/dom.ts";

const subsFile = process.argv[2];
if (!subsFile) {
  console.error("usage: node e2e/push.mjs <path to push-data/subscriptions.json>");
  process.exit(2);
}
const artifact = readFileSync(new URL("../../libs/rwire/assets/runtime.min.js", import.meta.url), "utf8");
const { document } = makeDom();

// The mock DOM lacks dispatchEvent and a real closest(): add both for the
// elements the runtime creates (what the extension relies on).
const patch = (el) => {
  el.dispatchEvent = (e) => el.fire(e.type, { target: el });
  el.closest = (sel) => {
    const attr = /^\[([\w-]+)\]$/.exec(sel)?.[1];
    for (let n = el; n; n = n.parentNode) if (attr && n.hasAttribute?.(attr)) return n;
    return null;
  };
  return el;
};
const ce = document.createElement, cens = document.createElementNS;
document.createElement = (t) => patch(ce(t));
document.createElementNS = (ns, t) => patch(cens(ns, t));

// The fake Push API.
const b64u = (b) => Buffer.from(b).toString("base64url");
let current = null;
let prompts = 0;
const env = {
  push: true,
  notif: {
    permission: "default",
    requestPermission() {
      prompts++;
      env.notif.permission = "granted";
      return Promise.resolve("granted");
    },
  },
  sw: {
    ready: Promise.resolve({
      pushManager: {
        getSubscription: async () => current,
        subscribe: async (o) => {
          const ecdh = createECDH("prime256v1");
          ecdh.generateKeys();
          const endpoint = "https://fcm.googleapis.com/fcm/send/e2e-" + b64u(randomBytes(9));
          const keys = { p256dh: b64u(ecdh.getPublicKey()), auth: b64u(randomBytes(16)) };
          current = {
            endpoint,
            options: { applicationServerKey: o.applicationServerKey.buffer },
            toJSON: () => ({ endpoint, expirationTime: null, keys }),
            unsubscribe: async () => ((current = null), true),
          };
          return current;
        },
      },
    }),
  },
};

const imports = [];
const pushExt = await import("../src/ext/push.ts");
const g = {
  __rwImport: (u) => {
    imports.push(u);
    return Promise.resolve({ v: pushExt.v, i: (d) => pushExt.i(d, env) });
  },
};

const noop = () => {};
class MO { observe() {} disconnect() {} }
const location = { protocol: "http:", host: "127.0.0.1:9010", pathname: "/", hash: "" };
const history = { pushState: noop, replaceState: noop };
const navigator = { onLine: true, clipboard: { writeText: noop } };
const factory = new Function(
  "document", "window", "addEventListener", "removeEventListener",
  "history", "location", "navigator", "WebSocket", "MutationObserver", "console",
  "setTimeout", "clearTimeout", "scrollTo", "globalThis", "BASE", "fetch",
  artifact,
);
factory(
  document, { addEventListener: noop }, noop, noop,
  history, location, navigator, WebSocket, MO, console,
  setTimeout, clearTimeout, noop, g, "", () => Promise.reject(new Error("no")),
);

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const walk = (n, out = []) => {
  out.push(n);
  for (const c of n.children || []) walk(c, out);
  return out;
};
const find = (pred) => walk(document.body).find(pred);
const button = (text) => find((n) => n.tagName === "BUTTON" && n.textContent.includes(text));
const body = () => document.body.textContent;
const stored = () => {
  try {
    return JSON.parse(readFileSync(subsFile, "utf8")).map((s) => s.endpoint);
  } catch {
    return [];
  }
};
const fail = (why) => {
  console.error("E2E FAIL: " + why + "\nbody: " + body().slice(0, 300));
  process.exit(1);
};

await sleep(1500); // initial render, extension load, first status report
if (!imports.some((u) => u.startsWith("/_rw/ext/push.js"))) fail("push extension not hinted: " + imports);
if (!body().includes("Notifications are off")) fail("first status not rendered");

const on = button("Turn on notifications");
if (!on) fail("subscribe button missing");
on.fire("click", { target: on });
if (prompts !== 1) fail("the permission prompt must run inside the click");
await sleep(800);
if (!body().includes("Notifications are on")) fail("subscribed state not rendered");
const endpoint = current.endpoint;
if (!stored().includes(endpoint)) fail("subscription not stored in " + subsFile);
console.log("subscribed: " + endpoint.slice(0, 60) + "… stored");

const off = button("Turn off");
if (!off) fail("unsubscribe button missing");
off.fire("click", { target: off });
await sleep(800);
if (!body().includes("Notifications are off")) fail("unsubscribed state not rendered");
if (stored().includes(endpoint)) fail("subscription still stored");
console.log("unsubscribed: removed from the file");
console.log("E2E PASS: gesture → subscribe → report over the WebSocket → stored → re-rendered, and back");
process.exit(0);
