// Extension primitive (M1): MOD_DEF loader dedup, data-kbd vim scoping, and
// the vim skeleton's mode plumbing. The vim module is imported directly and
// installed on the mock document — the same contract the loader uses.
import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { makeDom, type MockDoc, type MockEl } from "./dom.ts";
import { x } from "../src/executor.ts";
import { st } from "../src/state.ts";
import { i as installVim } from "../src/ext/vim.ts";

let doc: MockDoc;
let imports: string[];

function bytes(...parts: number[][]): Uint8Array {
  return new Uint8Array(parts.flat());
}

beforeEach(() => {
  const dom = makeDom();
  doc = dom.document;
  (globalThis as any).document = doc;
  (globalThis as any).BASE = "";
  st.w = { send() {}, readyState: 1 } as any;
  imports = [];
  (globalThis as any).__rwImport = (u: string) => {
    imports.push(u);
    return Promise.resolve({});
  };
});

const MOD_DEF = 0x8b;

test("MOD_DEF imports each named module once per page", () => {
  const hint = (name: string) =>
    bytes([MOD_DEF, 1, name.length], [...new TextEncoder().encode(name)], [0xff]);
  x(hint("vim"), []);
  x(hint("vim"), []); // page-level dedup even if the server re-hints
  assert.deepEqual(imports, ["/_rw/ext/vim.js?v=0"]);
});

test("data-kbd hook yields to a vim-moded target outside insert", async () => {
  const { installRouter } = await import("../src/router.ts");
  (globalThis as any).window = { addEventListener: () => {} };
  (globalThis as any).history = { pushState() {}, replaceState() {} };
  installRouter();
  const cancel = doc.createElement("span");
  cancel.setAttribute("data-kbd", "escape");
  let clicked = 0;
  cancel.addEventListener("click", () => clicked++);
  doc.body.appendChild(cancel);
  const ta = doc.createElement("textarea");
  ta.setAttribute("data-vim", "normal");
  doc.body.appendChild(ta);
  const press = (target: MockEl) => {
    for (const fn of doc.listeners["keydown"] || [])
      fn({ key: "Escape", target, preventDefault: () => {} });
  };
  press(ta);
  assert.equal(clicked, 0, "normal-mode editor owns Escape");
  ta.setAttribute("data-vim", "insert");
  press(ta);
  assert.equal(clicked, 1, "insert mode falls through to data-kbd");
});

test("Escape on a vim target is always prevented (module-lag safety)", async () => {
  const { installRouter } = await import("../src/router.ts");
  (globalThis as any).window = { addEventListener: () => {} };
  (globalThis as any).history = { pushState() {}, replaceState() {} };
  installRouter();
  // NO [data-kbd=escape] element exists; module not loaded either.
  const ta = doc.createElement("textarea");
  ta.setAttribute("data-vim", "insert");
  doc.body.appendChild(ta);
  let prevented = 0;
  for (const fn of doc.listeners["keydown"] || [])
    fn({ key: "Escape", target: ta, preventDefault: () => prevented++ });
  assert.ok(prevented >= 1, "Firefox's native textarea revert suppressed");
});

test("vim skeleton: mode transitions + chip + printables swallowed", () => {
  installVim(doc as any);
  const ta = doc.createElement("textarea");
  ta.setAttribute("data-vim", "normal");
  doc.body.appendChild(ta);
  const chip = doc.createElement("span");
  chip.setAttribute("data-vim-chip", "1");
  doc.body.appendChild(chip);
  let prevented = 0;
  const press = (key: string) => {
    for (const fn of doc.listeners["keydown"] || [])
      fn({ key, target: ta, preventDefault: () => prevented++ });
  };
  press("x"); // printable in normal mode: swallowed, no mode change
  assert.equal(prevented, 1);
  assert.equal(ta.getAttribute("data-vim"), "normal");
  press("i");
  assert.equal(ta.getAttribute("data-vim"), "insert");
  assert.equal(chip.textContent, "INSERT");
  press("x"); // typing in insert: NOT prevented
  assert.equal(prevented, 2, "only the i transition prevented");
  press("Escape");
  assert.equal(ta.getAttribute("data-vim"), "normal");
  press("V");
  assert.equal(ta.getAttribute("data-vim"), "V");
  assert.equal(chip.textContent, "V-LINE");
  press("Escape");
  press("v");
  assert.equal(chip.textContent, "VISUAL");
});

test("ext/vim.min.js stays within its own budget", async () => {
  const { readFileSync } = await import("node:fs");
  const { gzipSync } = await import("node:zlib");
  const out = readFileSync(new URL("../dist/ext/vim.min.js", import.meta.url));
  assert.ok(out.length <= 9_000, `vim ext ${out.length} bytes exceeds 9000`);
  assert.ok(gzipSync(out).length <= 3_500, "vim ext gzip exceeds 3500");
});

test("Tab indents on [data-tab-insert] fields; Shift+Tab dedents", async () => {
  const { installRouter } = await import("../src/router.ts");
  (globalThis as any).window = { addEventListener: () => {} };
  (globalThis as any).history = { pushState() {}, replaceState() {} };
  installRouter();
  const ta = doc.createElement("textarea") as any;
  ta.setAttribute("data-tab-insert", "");
  ta.value = "line";
  ta.selectionStart = 2;
  ta.selectionEnd = 2;
  doc.body.appendChild(ta);
  let inputs = 0;
  ta.addEventListener("input", () => inputs++);
  let prevented = 0;
  const press = (o: Record<string, unknown> = {}) => {
    for (const fn of doc.listeners["keydown"] || [])
      fn({ key: "Tab", target: ta, preventDefault: () => prevented++, ...o });
  };
  press();
  assert.equal(ta.value, "li\tne", "tab char inserted at caret");
  assert.equal(ta.selectionStart, 3);
  assert.equal(prevented, 1, "focus move suppressed");
  assert.equal(inputs, 1, "synthetic input keeps echo/server in sync");
  // dedent strips LINE-LEADING indent only
  ta.value = "\tline";
  ta.setSelectionRange(3, 3);
  press({ shiftKey: true });
  assert.equal(ta.value, "line", "Shift+Tab removed the leading tab");
  assert.equal(ta.selectionStart, 2, "caret shifted with the removal");
  // spaces variant
  ta.setAttribute("data-tab-insert", "2");
  ta.value = "x";
  ta.setSelectionRange(0, 0);
  press();
  assert.equal(ta.value, "  x", "N spaces per the attr value");
  // vim normal mode owns Tab (guard returns before the insert)
  ta.setAttribute("data-vim", "normal");
  ta.value = "x";
  ta.setSelectionRange(0, 0);
  press();
  assert.equal(ta.value, "x", "no insert while vim owns the keys");
});
