// Node test for the DOM morph (me/mk) extracted verbatim from capsule_gen.rs,
// run against a minimal DOM mock. Verifies node reuse by id, reordering,
// insert/remove, binding-key swap, nested-region preservation, and text/attr
// updates — the correctness the morph relies on. Run: node tests/morph_test.mjs
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const src = readFileSync(join(here, "../src/capsule_gen.rs"), "utf8");

// Extract the two morph functions verbatim from the shipped runtime string.
function extract(name) {
  const start = src.indexOf(`function ${name}(`);
  if (start < 0) throw new Error(`function ${name} not found`);
  // Brace-match from the first '{' after the signature.
  let i = src.indexOf("{", start), depth = 0;
  for (let j = i; j < src.length; j++) {
    if (src[j] === "{") depth++;
    else if (src[j] === "}") { depth--; if (depth === 0) return src.slice(start, j + 1); }
  }
  throw new Error(`unbalanced ${name}`);
}
const morphSrc = `${extract("me")}\n${extract("mk")}\nreturn { me, mk };`;
const { me, mk } = new Function(morphSrc)();

// --- minimal DOM mock ---
class Txt {
  constructor(v) { this.nodeType = 3; this.nodeValue = v; this.parent = null; }
}
class El {
  constructor(tag, id) {
    this.nodeType = 1; this.tagName = tag.toUpperCase();
    this.children = []; this.attrs = new Map(); this.parent = null;
    this.__hk = undefined;
    if (id) this.setAttribute("id", id);
  }
  get id() { return this.attrs.get("id") || ""; }
  get firstChild() { return this.children[0] || null; }
  get nextSibling() {
    if (!this.parent) return null;
    const k = this.parent.children.indexOf(this);
    return this.parent.children[k + 1] || null;
  }
  get attributes() {
    const a = [...this.attrs].map(([name, value]) => ({ name, value }));
    a.length = a.length; return a;
  }
  getAttribute(n) { return this.attrs.has(n) ? this.attrs.get(n) : null; }
  setAttribute(n, v) { this.attrs.set(n, String(v)); }
  hasAttribute(n) { return this.attrs.has(n); }
  removeAttribute(n) { this.attrs.delete(n); }
  appendChild(c) { return this.insertBefore(c, null); }
  insertBefore(c, ref) {
    if (c.parent) c.parent.children.splice(c.parent.children.indexOf(c), 1);
    c.parent = this;
    const idx = ref ? this.children.indexOf(ref) : this.children.length;
    this.children.splice(idx < 0 ? this.children.length : idx, 0, c);
    return c;
  }
  removeChild(c) { this.children.splice(this.children.indexOf(c), 1); c.parent = null; return c; }
}
const t = (v) => new Txt(v);
function el(tag, id, attrs, hk) {
  const e = new El(tag, id);
  if (attrs) for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  if (hk !== undefined) e.__hk = hk;
  return e;
}
function tree(...kids) { const r = new El("div"); for (const k of kids) r.appendChild(k); return r; }
function ids(node) { return node.children.map((c) => c.nodeType === 1 ? c.id || c.tagName : `#${c.nodeValue}`); }

let pass = 0, fail = 0;
function ok(cond, msg) { if (cond) pass++; else { fail++; console.error("FAIL:", msg); } }

// T1: reuse by id (same object identity), update attribute
{
  const live = tree(el("input", "x", { value: "1" }));
  const liveInput = live.children[0];
  const shadow = tree(el("input", "x", { value: "2" }));
  mk(live, shadow);
  ok(live.children[0] === liveInput, "T1 input node reused (identity)");
  ok(liveInput.getAttribute("value") === "2", "T1 value attribute updated");
}

// T2: reorder by id reuses the same nodes
{
  const a = el("div", "a"), b = el("div", "b"), c = el("div", "c");
  const live = tree(a, b, c);
  const shadow = tree(el("div", "c"), el("div", "a"), el("div", "b"));
  mk(live, shadow);
  ok(JSON.stringify(ids(live)) === JSON.stringify(["c", "a", "b"]), "T2 reordered");
  ok(live.children[0] === c && live.children[1] === a && live.children[2] === b, "T2 nodes reused on reorder");
}

// T3: insert and remove
{
  const a = el("div", "a"), b = el("div", "b");
  const live = tree(a, b);
  const shadow = tree(el("div", "a"), el("div", "x"), el("div", "b"), el("div", "y"));
  mk(live, shadow);
  ok(JSON.stringify(ids(live)) === JSON.stringify(["a", "x", "b", "y"]), "T3 inserted x,y");
  ok(live.children[0] === a && live.children[2] === b, "T3 kept a,b");
  const live2 = tree(el("div", "d"));
  mk(live2, tree());
  ok(live2.children.length === 0, "T3 removed all");
}

// T4: binding key changed -> node NOT reused (swapped to shadow's node)
{
  const oldBtn = el("button", "k", {}, "r1_5");
  const live = tree(oldBtn);
  const newBtn = el("button", "k", {}, "r1_9");
  mk(live, tree(newBtn));
  ok(live.children[0] === newBtn, "T4 changed-binding node swapped in");
  ok(live.children[0] !== oldBtn, "T4 stale-binding node dropped");
}

// T5: binding key unchanged -> node reused (listener preserved)
{
  const btn = el("button", "k", {}, "p1_5_3,4");
  const live = tree(btn);
  mk(live, tree(el("button", "k", {}, "p1_5_3,4")));
  ok(live.children[0] === btn, "T5 unchanged-binding node reused");
}

// T6: nested __synced_ wrapper preserved (children untouched)
{
  const nested = el("span", "__synced_3");
  nested.appendChild(el("b", "inner"));
  const live = tree(nested);
  mk(live, tree(el("span", "__synced_3"))); // shadow nested is empty
  ok(live.children[0] === nested, "T6 nested wrapper reused");
  ok(nested.children.length === 1 && nested.children[0].id === "inner", "T6 nested content preserved");
}

// T7: text content update on a reused element
{
  const div = el("div", "d");
  div.appendChild(t("old"));
  const live = tree(div);
  const sdiv = el("div", "d");
  sdiv.appendChild(t("new"));
  mk(live, tree(sdiv));
  ok(live.children[0] === div, "T7 div reused");
  ok(div.children[0].nodeValue === "new", "T7 text updated");
}

console.log(`morph test: ${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
