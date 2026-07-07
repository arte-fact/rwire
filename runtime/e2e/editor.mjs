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
  setTimeout, clearTimeout, noop,
  { __rwImport: () => import(new URL("../dist/ext/vim.min.js", import.meta.url)) }, "",
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

// 0. The synced wrapper is a styled flex item (the full-height fix).
const wrapper = find((n) => n.id && n.id.startsWith("__synced_") && (n.classList?.all() || []).length);
if (!wrapper) fail("synced wrapper has no style tokens — region would collapse");

// 1. Select README.md → rendered markdown view.
const row = find((n) => n.id === "tn-README.md");
if (!row) fail("tree row for README.md missing");
row.fire("click", { target: row });
await sleep(600);
if (!document.body.textContent.includes("Sample workspace"))
  fail("rendered markdown view missing");

// 2. Edit mode.
const editBtn = find((n) => n.getAttribute?.("aria-label") === "Edit" && clickable(n));
if (!editBtn) fail("Edit icon button missing");
editBtn.fire("click", { target: editBtn });
await sleep(600);
const ta = find((n) => n.tagName === "TEXTAREA");
if (!ta) fail("editor textarea missing");
if (find((n) => (n.getAttribute?.("aria-label") || "").startsWith("Save") && clickable(n)))
  fail("no Save button expected while autosave is on");
// Syntax overlay: colored underlay present; keystrokes echo instantly.
const underlay = find((n) => (n.id || "").endsWith("-hl"));
if (!underlay) fail("syntax overlay underlay missing");
if (ta.getAttribute("data-echo") !== underlay.id) fail("textarea missing data-echo");
if (!ta.getAttribute("rows")) fail("rows fallback missing (field-sizing-less browsers)");
ta.value = "echo-check";
ta.fire("input", { target: ta });
if (underlay.textContent !== "echo-check") fail("overlay echo not instant");
await sleep(900); // colors return with the round-trip morph
if (!(underlay.children || []).length) fail("overlay lines not restored by morph");
ta.value = original;
ta.fire("input", { target: ta });
await sleep(900);

// 3. Type → autosave flushes to disk with zero clicks.
const stamp = `autosaved-by-e2e-${Date.now()}`;
ta.value = original + "\n" + stamp + "\n";
ta.fire("input", { target: ta });
await sleep(1000); // 250ms client debounce + round-trip + server flush
if (!disk().includes(stamp)) fail("autosave did not reach the disk");
if (!document.body.textContent.includes("saved · auto"))
  fail("status bar missing 'saved · auto'");

// 3b. Undo reverts the edit (and autosave flushes it); Redo restores.
const undoBtn = find((n) => (n.getAttribute?.("aria-label") || "").startsWith("Undo") && clickable(n));
if (!undoBtn) fail("Undo button missing");
undoBtn.fire("click", { target: undoBtn });
await sleep(700);
if (disk() !== original) fail("undo did not revert the disk");
const redoBtn = find((n) => (n.getAttribute?.("aria-label") || "").startsWith("Redo") && clickable(n));
if (!redoBtn) fail("Redo button missing (or not enabled after undo)");
redoBtn.fire("click", { target: redoBtn });
await sleep(700);
if (!disk().includes(stamp)) fail("redo did not restore the edit");
// the textarea was re-keyed by undo/redo — re-find it for later steps
if (!find((n) => n.tagName === "TEXTAREA")) fail("re-keyed textarea missing");

// 4. Toggle autosave off → edits stay local until the Save button.
const toggle = find((n) => n.textContent.includes("autosave") && clickable(n));
if (!toggle) fail("autosave toggle missing");
const checkboxOn = walk(toggle).find((n) => n.tagName === "INPUT" && n.getAttribute("type") === "checkbox");
if (!checkboxOn || checkboxOn.getAttribute("checked") === null)
  fail("switch should render checked while autosave is on");
checkboxOn.checked = true; // as the browser would after native rendering
toggle.fire("click", { target: toggle });
await sleep(600);
const toggle2 = find((n) => n.textContent.includes("autosave") && clickable(n));
const checkboxOff = walk(toggle2).find((n) => n.tagName === "INPUT" && n.getAttribute("type") === "checkbox");
if (!checkboxOff) fail("switch disappeared after toggle");
if (checkboxOff.getAttribute("checked") !== null || checkboxOff.checked === true)
  fail("switch did not visually flip off (attr/property stale)");
const ta2 = find((n) => n.tagName === "TEXTAREA");
const stamp2 = `manually-saved-${Date.now()}`;
ta2.value = ta2.value ? ta2.value + stamp2 + "\n" : original + "\n" + stamp + "\n" + stamp2 + "\n";
ta2.fire("input", { target: ta2 });
await sleep(900);
if (disk().includes(stamp2)) fail("manual mode must not auto-persist");
const saveBtn = find((n) => n.getAttribute?.("aria-label") === "Save · ⌘S" && clickable(n));
if (!saveBtn) fail("Save icon button missing in manual mode");
if (saveBtn.getAttribute("data-kbd") !== "mod+s") fail("Save button missing data-kbd");
// save via the KEYBOARD path: Ctrl+S → [data-kbd=mod+s] click → binding
for (const fn of document.listeners["keydown"] || [])
  fn({ key: "s", ctrlKey: true, target: ta2, preventDefault: () => {} });
await sleep(700);
if (!disk().includes(stamp2)) fail("Ctrl+S save did not reach the disk");

// 4c. Vim mode: toggle on → lazy module loads → dd deletes a line through
// the whole stack (module → synthetic input → Edit → autosave → disk) → u
// delegates to the server undo.
const autosaveToggle = find((n) => n.textContent.includes("autosave") && clickable(n));
autosaveToggle.fire("click", { target: autosaveToggle }); // back on for vim writes
await sleep(500);
const vimToggle = find((n) => n.textContent === "vim" && (n.parentNode?.__b?.click || n.__b?.click));
const vimClickable = walk(document.body).find(
  (n) => clickable(n) && (n.textContent || "").startsWith("vim"),
);
if (!vimClickable) fail("vim toggle missing");
vimClickable.fire("click", { target: vimClickable });
await sleep(800); // round trip + module import
const vta = find((n) => n.tagName === "TEXTAREA");
if (!vta) fail("textarea missing after vim toggle");
if (vta.getAttribute("data-vim") !== "normal") fail("data-vim entry mode missing");
if (!find((n) => n.getAttribute?.("data-vim-chip"))) fail("vim mode chip missing");
const before = disk();
const press = (key, o = {}) => {
  for (const fn of document.listeners["keydown"] || [])
    fn({ key, target: vta, preventDefault: () => {}, ...o });
};
vta.setSelectionRange(0, 0);
press("d"); press("d");
await sleep(900);
if (disk() === before) fail("vim dd did not reach the disk");
if (disk().split("\n").length >= before.split("\n").length)
  fail("vim dd did not remove a line");
press("u");
await sleep(900);
if (disk() !== before) fail("vim u did not delegate to server undo");
// leave vim + restore autosave state for the next steps
vimClickable.fire("click", { target: vimClickable });
await sleep(500);

// 5. Code view: gutter-aligned colored lines (not the md code block).
const rsRow = find((n) => n.id === "tn-src/main.rs");
if (!rsRow) fail("main.rs tree row missing");
// switching files may trigger the unsaved guard; discard if it appears
rsRow.fire("click", { target: rsRow });
await sleep(500);
const guard = find((n) => n.textContent === "Discard" && clickable(n));
if (guard) {
  guard.fire("click", { target: guard });
  await sleep(500);
}
if (!document.body.textContent.includes("fn main")) fail("code view missing content");
if (find((n) => n.tagName === "PRE"))
  fail("code view must not use the markdown <pre> block");
if (!document.body.textContent.match(/\b3\b/)) fail("code view gutter numbers missing");

writeFileSync(samplePath, original);
console.log("E2E PASS: autosave hands-free · switch flips visually · manual gated · gutter code view");
process.exit(0);
