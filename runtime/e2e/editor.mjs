// Full-stack E2E (manual): the shipped artifact against a LIVE editor server.
// Start it first: `cargo run -p editor`, then `node e2e/editor.mjs` from
// runtime/. Flow: select README.md in the tree → rendered view arrives →
// Edit → textarea with gutter → type (debounced input → dirty diff → Save
// enables) → Save → assert THE FILE ON DISK changed, then restore it.
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
const fail = (why) => {
  writeFileSync(samplePath, original);
  console.error("E2E FAIL:", why);
  process.exit(1);
};

await sleep(1200);

// 1. Select README.md in the tree.
const row = find((n) => n.id === "tn-README.md");
if (!row) fail("tree row for README.md missing");
row.fire("click", { target: row });
await sleep(600);
if (!document.body.textContent.includes("Sample workspace"))
  fail("rendered markdown view missing");

// 2. Switch to Edit.
const editChip = find(
  (n) => n.textContent === "Edit" && (n.listeners?.click || []).length,
);
if (!editChip) fail("Edit chip missing");
editChip.fire("click", { target: editChip });
await sleep(600);
const ta = find((n) => n.tagName === "TEXTAREA");
if (!ta) fail("editor textarea missing");
if (!document.body.textContent.includes("2")) fail("gutter line numbers missing");
const savedBtn = find((n) => n.tagName === "BUTTON" && n.textContent === "Saved");
if (!savedBtn || savedBtn.getAttribute("disabled") === null)
  fail("save button should start disabled/Saved");

// 3. Type: working copy diverges → dirty → Save enables.
const stamp = `edited-by-e2e-${Date.now()}`;
ta.value = original + "\n" + stamp + "\n";
ta.fire("input", { target: ta });
await sleep(900); // 250ms debounce + round-trip + re-render
const saveBtn = find((n) => n.tagName === "BUTTON" && n.textContent === "Save");
if (!saveBtn) fail("Save button did not enable after edits");
if (saveBtn.getAttribute("disabled") !== null) fail("Save button still disabled");

// 4. Save → the file on disk must contain the edit.
saveBtn.fire("click", { target: saveBtn });
await sleep(700);
const onDisk = readFileSync(samplePath, "utf8");
if (!onDisk.includes(stamp)) fail("save did not reach the disk");
const savedAgain = find((n) => n.tagName === "BUTTON" && n.textContent === "Saved");
if (!savedAgain) fail("button did not return to Saved after saving");

writeFileSync(samplePath, original); // restore the sample
console.log("E2E PASS: browse -> view -> edit -> dirty -> save hit the disk");
process.exit(0);
