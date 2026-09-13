// Full-stack E2E (manual): the shipped artifact + a real WebSocket against a
// LIVE counter server. Start it first: `cargo run -p counter`, then
// `node e2e/counter.mjs` from runtime/. The bundle bootstraps itself
// (connect → initial render into a mock DOM); we fire the +/- buttons'
// captured listeners and assert each server round-trip patches the count.
import { readFileSync } from "node:fs";
import { makeDom } from "../test/dom.ts";

const artifact = readFileSync(new URL("../../libs/rwire/assets/runtime.min.js", import.meta.url), "utf8");
const { document } = makeDom();

const noop = () => {};
class MO { observe() {} disconnect() {} }
const location = { protocol: "http:", host: "127.0.0.1:9000", pathname: "/", hash: "" };
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
  setTimeout, clearTimeout, noop, {}, "", () => Promise.reject(new Error("no")),
);

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const walk = (n, out = []) => {
  out.push(n);
  for (const c of n.children || []) walk(c, out);
  return out;
};
const find = (pred) => walk(document.body).find(pred);

await sleep(1200); // initial render
const count = () => find((n) => n.tagName === "H1")?.textContent;
const plus = find((n) => n.tagName === "BUTTON" && n.textContent === "+");
const minus = find((n) => n.tagName === "BUTTON" && n.textContent === "-");
if (!plus || !minus) {
  console.error("E2E FAIL: buttons not found; body:", document.body.textContent.slice(0, 200));
  process.exit(1);
}
const c0 = count();

plus.fire("click");
await sleep(500);
const c1 = count();
plus.fire("click");
await sleep(500);
const c2 = count();
minus.fire("click");
await sleep(500);
const c3 = count();

console.log(`counts: ${c0} -> ${c1} -> ${c2} -> ${c3}`);
const n0 = Number(c0);
const ok = Number(c1) === n0 + 1 && Number(c2) === n0 + 2 && Number(c3) === n0 + 1;
console.log(ok ? "E2E PASS: live server round-trips patch the DOM" : "E2E FAIL");
process.exit(ok ? 0 : 1);
