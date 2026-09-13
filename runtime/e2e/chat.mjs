// Full-stack multi-client E2E (manual): TWO sandboxed instances of the shipped
// artifact, each with its own real WebSocket to a LIVE chatroom server —
// proving shared-state broadcast end to end. Start the server first:
// `cargo run -p chat`, then `node e2e/chat.mjs` from runtime/.
//
// Flow: both clients join with nicknames; alice types (draft → typing
// indicator broadcast), sends a message; assert BOB's DOM shows it (the
// cross-connection hop), then bob replies and alice sees it.
import { readFileSync } from "node:fs";
import { makeDom } from "../test/dom.ts";

const artifact = readFileSync(
  new URL("../../libs/rwire/assets/runtime.min.js", import.meta.url),
  "utf8",
);

function client() {
  const { document } = makeDom();
  const noop = () => {};
  class MO { observe() {} disconnect() {} }
  class IO { observe() {} disconnect() {} }
  const location = { protocol: "http:", host: "127.0.0.1:9007", pathname: "/", hash: "" };
  const factory = new Function(
    "document", "window", "addEventListener", "removeEventListener",
    "history", "location", "navigator", "WebSocket", "MutationObserver",
    "IntersectionObserver", "console",
    "setTimeout", "clearTimeout", "scrollTo", "globalThis", "BASE", "fetch",
    artifact,
  );
  factory(
    document, { addEventListener: noop }, noop, noop,
    { pushState: noop, replaceState: noop }, location,
    { onLine: true, clipboard: { writeText: noop } },
    WebSocket, MO, IO, console,
    setTimeout, clearTimeout, noop, {}, "",
    () => Promise.reject(new Error("no")),
  );
  const walk = (n, out = []) => {
    out.push(n);
    for (const c of n.children || []) walk(c, out);
    return out;
  };
  return {
    document,
    find: (pred) => walk(document.body).find(pred),
    text: () => document.body.textContent,
  };
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// The example's forms submit with FormData(form) in gp(); give the sandbox a
// FormData that reads named fields from the mock tree.
globalThis.FormData = class {
  constructor(form) {
    this.fields = [];
    const walk = (n) => {
      const name = n.getAttribute?.("name");
      if (name) this.fields.push([name, n.value || n.getAttribute("value") || ""]);
      for (const c of n.children || []) walk(c);
    };
    walk(form);
  }
  forEach(cb) {
    for (const [k, v] of this.fields) cb(v, k);
  }
};

function join(c, nick) {
  const input = c.find((n) => n.getAttribute?.("name") === "name" && n.tagName === "INPUT");
  input.value = nick;
  const form = c.find((n) => n.tagName === "FORM");
  form.fire("submit", { target: form });
}

function send(c, text) {
  const ta = c.find((n) => n.tagName === "TEXTAREA");
  ta.value = text;
  const form = c.find(
    (n) => n.tagName === "FORM" && c.find((m) => m.tagName === "TEXTAREA"),
  );
  ta.fire("input", { target: ta }); // draft → typing indicator (debounced 400ms)
  const f = ta.parentNode?.tagName === "FORM" ? ta.parentNode : form;
  f.fire("submit", { target: f });
}

const alice = client();
const bob = client();
await sleep(1200); // both connected + initial render

const fail = (why) => {
  console.error("E2E FAIL:", why);
  process.exit(1);
};

if (!alice.text().includes("No messages yet")) fail("empty room state missing");

join(alice, "alice");
join(bob, "bob");
await sleep(600);
if (!alice.find((n) => n.tagName === "TEXTAREA")) fail("alice composer missing after join");
if (!bob.find((n) => n.tagName === "TEXTAREA")) fail("bob composer missing after join");

// alice drafts (typing indicator must reach BOB), then sends.
const ta = alice.find((n) => n.tagName === "TEXTAREA");
ta.value = "hello from alice";
ta.fire("input", { target: ta });
await sleep(900); // 400ms debounce + broadcast
const bobSawTyping = bob.text().includes("typing");
const aform = ta.parentNode.tagName === "FORM" ? ta.parentNode : alice.find((n) => n.tagName === "FORM");
aform.fire("submit", { target: aform });
await sleep(700);

if (!bob.text().includes("hello from alice")) fail("bob did not receive alice's message");
if (!bob.text().includes("alice")) fail("bob missing author name");

// bob replies; alice must see it.
const tb = bob.find((n) => n.tagName === "TEXTAREA");
tb.value = "hi alice, bob here";
const bform = tb.parentNode.tagName === "FORM" ? tb.parentNode : bob.find((n) => n.tagName === "FORM");
bform.fire("submit", { target: bform });
await sleep(700);

if (!alice.text().includes("hi alice, bob here")) fail("alice did not receive bob's reply");

console.log(`typing indicator crossed connections: ${bobSawTyping ? "yes" : "NO"}`);
console.log("E2E PASS: two live clients converse through shared state");
process.exit(bobSawTyping ? 0 : 1);
