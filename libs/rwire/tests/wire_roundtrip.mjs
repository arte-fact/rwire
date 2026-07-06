// Wire round-trip harness: runs the REAL runtime opcode parser `x()` (the
// built bundle the capsule embeds) over byte streams emitted by the Rust
// encoder, to catch wire desyncs (the parser walking off the rails on a
// length/symbol field).
//
// A desync surfaces as a console PARSE ERROR / "Unknown opcode" inside x(); a clean
// stream parses every opcode and ends at BATCH_END (0xFF) consuming all bytes.
//
// Runtime source: the vendored artifact libs/rwire/assets/runtime.min.js —
// exactly what ships — so `cargo test --test wire_roundtrip` doubles as the
// local staleness gate. Override with RWIRE_RUNTIME=<path> (the Rust wrapper
// inherits env) to drive a fresh runtime/dist build before syncing.
//
// Usage:
//   node tests/wire_roundtrip.mjs <fixture-dir>   # parse every *.bin, exit 1 on any error
//   (also importable: `import { runWire } from "./wire_roundtrip.mjs"`)
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const ARTIFACT = process.env.RWIRE_RUNTIME || join(here, "../assets/runtime.min.js");
const CLIENT_JS = readFileSync(ARTIFACT, "utf8");

// --- lenient DOM mock: operations must not throw, so x() can walk the whole
// stream; a desync then shows up as an unknown opcode / out-of-range read, not as
// an incidental DOM TypeError. ---
function makeDom() {
  const byId = new Map();
  let uid = 0;
  const mkClassList = () => {
    const set = new Set();
    return { add: (c) => set.add(c), remove: (c) => set.delete(c), contains: (c) => set.has(c), _set: set };
  };
  function el(tag) {
    const node = {
      nodeType: 1,
      tagName: String(tag || "div").toUpperCase(),
      _attrs: new Map(),
      _children: [],
      parentNode: null,
      classList: mkClassList(),
      style: {},
      dataset: {},
      __hk: undefined,
      __t: undefined,
      _value: "",
      _uid: ++uid,
      get id() { return this._attrs.get("id") || ""; },
      set id(v) { this._attrs.set("id", v); byId.set(v, this); },
      get className() { return this._attrs.get("class") || ""; },
      set className(v) { this._attrs.set("class", v); },
      get textContent() { return this._text || ""; },
      set textContent(v) { this._text = String(v); this._children = []; },
      get value() { return this._value; },
      set value(v) { this._value = v; },
      setAttribute(n, v) { this._attrs.set(n, String(v)); if (n === "id") byId.set(String(v), this); },
      getAttribute(n) { return this._attrs.has(n) ? this._attrs.get(n) : null; },
      removeAttribute(n) { this._attrs.delete(n); },
      hasAttribute(n) { return this._attrs.has(n); },
      get attributes() { return [...this._attrs].map(([name, value]) => ({ name, value })); },
      appendChild(c) { c.parentNode = this; this._children.push(c); return c; },
      insertBefore(c, ref) {
        c.parentNode = this;
        const idx = ref ? this._children.indexOf(ref) : -1;
        if (idx < 0) this._children.push(c); else this._children.splice(idx, 0, c);
        return c;
      },
      removeChild(c) { const i = this._children.indexOf(c); if (i >= 0) this._children.splice(i, 1); c.parentNode = null; return c; },
      replaceChild(n2, o) { const i = this._children.indexOf(o); if (i >= 0) this._children[i] = n2; n2.parentNode = this; return o; },
      get firstChild() { return this._children[0] || null; },
      get childNodes() { return this._children; },
      get children() { return this._children.filter((c) => c.nodeType === 1); },
      get nextSibling() {
        if (!this.parentNode) return null;
        const ch = this.parentNode._children, i = ch.indexOf(this);
        return i >= 0 && i + 1 < ch.length ? ch[i + 1] : null;
      },
      addEventListener() {},
      removeEventListener() {},
      focus() {},
      setSelectionRange() {},
      selectionStart: 0,
      selectionEnd: 0,
      querySelectorAll() { return []; },
      querySelector() { return null; },
      cloneNode() { return el(this.tagName); },
    };
    return node;
  }
  const body = el("body");
  const head = el("head");
  const document = {
    body,
    head,
    activeElement: null,
    createElement: (t) => el(t),
    createElementNS: (_ns, t) => el(t),
    createTextNode: (v) => ({ nodeType: 3, nodeValue: String(v), textContent: String(v), parentNode: null }),
    getElementById: (id) => {
      if (byId.has(id)) return byId.get(id);
      // Auto-vivify so GET_BY_ID/GET_SYNCED never yield null (we test parsing, not lookup).
      const n = el("div"); n.id = id; return n;
    },
    addEventListener() {},
    head_appended: [],
  };
  return { document, body, byId };
}

// Build the runnable module: assembled client JS + lenient stubs. The runtime's
// top-level bootstrap (connect(), event listeners) runs harmlessly against stubs;
// we then call the now-defined `x` on each fixture.
function buildRuntime() {
  const { document } = makeDom();
  const noop = () => {};
  const wsStub = function () { return { binaryType: "", send: noop, close: noop, readyState: 1 }; };
  const historyStub = { pushState: noop, replaceState: noop, scrollRestoration: "auto" };
  const locationStub = { protocol: "http:", host: "localhost", pathname: "/", hash: "" };
  const navigatorStub = { onLine: true, serviceWorker: undefined, clipboard: { writeText: noop } };
  class MO { observe() {} disconnect() {} }

  const errors = [];
  const captureConsole = {
    error: (...a) => errors.push(a.map(String).join(" ")),
    log: noop, warn: noop,
  };

  // The bundle's `globalThis.__rwx = x` hook exposes the executor; shadowing
  // `globalThis` with a fresh object isolates it per run. `BASE` is injected
  // as the capsule would.
  const factory = new Function(
    "document", "window", "addEventListener", "removeEventListener",
    "history", "location", "navigator", "WebSocket", "MutationObserver", "console",
    "setTimeout", "clearTimeout", "scrollTo", "globalThis", "BASE",
    `${CLIENT_JS}\n;return { x: globalThis.__rwx };`
  );

  const win = { addEventListener: noop, removeEventListener: noop };
  const mod = factory(
    document, win, noop, noop,
    historyStub, locationStub, navigatorStub, wsStub, MO, captureConsole,
    (fn) => 0, noop, noop,
    {}, ""
  );
  return { x: mod.x, errors, document };
}

// Parse one byte stream; returns { ok, errors }.
export function runWire(bytes) {
  const rt = buildRuntime();
  rt.x(new Uint8Array(bytes));
  const errors = rt.errors.filter((e) => /PARSE ERROR|Unknown opcode/.test(e));
  return { ok: errors.length === 0, errors, allLogs: rt.errors };
}

// --- CLI: parse every fixture in a directory ---
function main() {
  const dir = process.argv[2];
  if (!dir) { console.error("usage: wire_roundtrip.mjs <fixture-dir>"); process.exit(2); }
  const files = readdirSync(dir).filter((f) => f.endsWith(".bin")).sort();
  if (files.length === 0) { console.error(`no .bin fixtures in ${dir}`); process.exit(2); }
  let failed = 0;
  for (const f of files) {
    const bytes = readFileSync(join(dir, f));
    const { ok, errors } = runWire(bytes);
    if (ok) {
      console.log(`ok    ${f}  (${bytes.length} bytes)`);
    } else {
      failed++;
      console.log(`FAIL  ${f}  (${bytes.length} bytes)`);
      for (const e of errors) console.log(`        ${e}`);
    }
  }
  console.log(`\n${files.length - failed}/${files.length} fixtures parsed cleanly`);
  process.exit(failed ? 1 : 0);
}

if (import.meta.url === `file://${process.argv[1]}`) main();
