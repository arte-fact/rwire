// Full-stack E2E (manual): the FileEditor kit against a LIVE editor server.
// Start it first: `cargo run -p editor`, then `node e2e/editor.mjs` from
// runtime/. Flow: select README.md → rendered view → Edit → type → AUTOSAVE
// flushes to disk with no clicks → toggle autosave off → type → nothing
// persists until the Save button → click Save → disk. Restores the sample.
import { readFileSync, writeFileSync } from "node:fs";
import { makeDom } from "../test/dom.ts";

const artifact = readFileSync(
  new URL("../../libs/rwire/assets/runtime.min.js", import.meta.url),
  "utf8",
);
const samplePath = new URL(
  "../../examples/editor/sample/README.md",
  import.meta.url,
);
const original = readFileSync(samplePath, "utf8");
const disk = () => readFileSync(samplePath, "utf8");

const { document } = makeDom();
const noop = () => {};
class MO { observe() {} disconnect() {} }
class IO { observe() {} disconnect() {} }
const factory = new Function(
  "document", "window", "addEventListener", "removeEventListener",
  "history", "location", "navigator", "WebSocket", "MutationObserver",
  "IntersectionObserver", "console",
  "setTimeout", "clearTimeout", "scrollTo", "globalThis", "BASE", "fetch",
  artifact,
);
factory(
  document, { addEventListener: noop }, noop, noop,
  { pushState: noop, replaceState: noop },
  { protocol: "http:", host: "127.0.0.1:9008", pathname: "/", hash: "" },
  { onLine: true, clipboard: { writeText: noop } },
  WebSocket, MO, IO, console,
  setTimeout, clearTimeout, noop, {}, "",
  () => Promise.reject(new Error("no")),
);

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const walk = (n, out = []) => {
  out.push(n);
  for (const c of n.children || []) walk(c, out);
  return out;
};
const find = (pred) => walk(document.body).find(pred);
const clickable = (n) => (n.__b?.click?.length || 0) > 0;
const fail = (why) => {
  writeFileSync(samplePath, original);
  console.error("E2E FAIL:", why);
  process.exit(1);
};

await sleep(1200);

// 1. Select README.md → rendered markdown view.
const row = find((n) => n.id === "tn-README.md");
if (!row) fail("tree row for README.md missing");
row.fire("click", { target: row });
await sleep(600);
if (!document.body.textContent.includes("Sample workspace"))
  fail("rendered markdown view missing");

// 2. Edit mode.
const editChip = find((n) => n.textContent === "Edit" && clickable(n));
if (!editChip) fail("Edit chip missing");
editChip.fire("click", { target: editChip });
await sleep(600);
const ta = find((n) => n.tagName === "TEXTAREA");
if (!ta) fail("editor textarea missing");
if (find((n) => n.tagName === "BUTTON" && n.textContent.includes("Save")))
  fail("no Save button expected while autosave is on");

// 3. Type → autosave flushes to disk with zero clicks.
const stamp = `autosaved-by-e2e-${Date.now()}`;
ta.value = original + "\n" + stamp + "\n";
ta.fire("input", { target: ta });
await sleep(1000); // 250ms client debounce + round-trip + server flush
if (!disk().includes(stamp)) fail("autosave did not reach the disk");
if (!document.body.textContent.includes("saved · auto"))
  fail("status bar missing 'saved · auto'");

// 4. Toggle autosave off → edits stay local until the Save button.
const toggle = find((n) => n.textContent.includes("autosave") && clickable(n));
if (!toggle) fail("autosave toggle missing");
toggle.fire("click", { target: toggle });
await sleep(600);
const ta2 = find((n) => n.tagName === "TEXTAREA");
const stamp2 = `manually-saved-${Date.now()}`;
ta2.value = ta2.value ? ta2.value + stamp2 + "\n" : original + "\n" + stamp + "\n" + stamp2 + "\n";
ta2.fire("input", { target: ta2 });
await sleep(900);
if (disk().includes(stamp2)) fail("manual mode must not auto-persist");
const saveBtn = find((n) => n.tagName === "BUTTON" && n.textContent.includes("Save ⌘S"));
if (!saveBtn) fail("Save button missing in manual mode");
saveBtn.fire("click", { target: saveBtn });
await sleep(700);
if (!disk().includes(stamp2)) fail("manual save did not reach the disk");

writeFileSync(samplePath, original);
console.log("E2E PASS: autosave flushed hands-free; manual mode gated on Save");
process.exit(0);
