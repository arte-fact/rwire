import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { me, mk, fm } from "../src/morph.ts";
import { st } from "../src/state.ts";
import { makeDom, type MockEl } from "./dom.ts";

let doc: ReturnType<typeof makeDom>["document"];
beforeEach(() => {
  doc = makeDom().document;
  st.pm = null;
});

const div = (id?: string) => {
  const e = doc.createElement("div");
  if (id) e.id = id;
  return e;
};

test("me syncs attributes both ways and copies __hk", () => {
  const a = div(),
    b = div();
  a.setAttribute("stale", "1");
  b.setAttribute("fresh", "2");
  b.__hk = "r1_2";
  me(a as any, b as any);
  assert.equal(a.getAttribute("stale"), null);
  assert.equal(a.getAttribute("fresh"), "2");
  assert.equal(a.__hk, "r1_2");
});

test("mk reuses nodes by id across reorder", () => {
  const live = div();
  const x = div("x"),
    y = div("y");
  live.appendChild(x);
  live.appendChild(y);
  const shadow = div();
  shadow.appendChild(div("y"));
  shadow.appendChild(div("x"));
  mk(live as any, shadow as any);
  assert.deepEqual(
    live.children.map((c: MockEl) => c.id),
    ["y", "x"],
  );
  assert.equal(live.children[1], x, "same node object reused");
  assert.equal(live.children[0], y, "same node object reused");
});

test("mk reuses id-less same-tag nodes positionally", () => {
  const live = div();
  const p = doc.createElement("p");
  p.textContent = "old";
  live.appendChild(p);
  const shadow = div();
  const p2 = doc.createElement("p");
  p2.textContent = "new";
  shadow.appendChild(p2);
  mk(live as any, shadow as any);
  assert.equal(live.children[0], p, "node reused");
  assert.equal(live.children[0].textContent, "new");
});

test("a changed __hk disqualifies reuse (fresh listener wins)", () => {
  const live = div();
  const btn = doc.createElement("button");
  btn.__hk = "r1_1";
  live.appendChild(btn);
  const shadow = div();
  const btn2 = doc.createElement("button");
  btn2.__hk = "r1_9";
  shadow.appendChild(btn2);
  mk(live as any, shadow as any);
  assert.equal(live.children[0], btn2, "replaced, not morphed");
});

test("me does not descend into nested __synced_ regions", () => {
  const a = div("__synced_7");
  const inner = div();
  inner.textContent = "keep me";
  a.appendChild(inner);
  const b = div("__synced_7"); // empty in the shadow
  me(a as any, b as any);
  assert.equal(a.children.length, 1, "children untouched");
  assert.equal(a.children[0].textContent, "keep me");
});

test("mk removes live extras and inserts shadow additions", () => {
  const live = div();
  live.appendChild(div("keep"));
  live.appendChild(div("drop"));
  const shadow = div();
  shadow.appendChild(div("keep"));
  shadow.appendChild(div("add"));
  mk(live as any, shadow as any);
  assert.deepEqual(
    live.children.map((c: MockEl) => c.id),
    ["keep", "add"],
  );
});

test("mk interleaves insertions among kept id-matched nodes", () => {
  const live = div();
  const a = div("a"),
    b = div("b");
  live.appendChild(a);
  live.appendChild(b);
  const shadow = div();
  for (const id of ["a", "x", "b", "y"]) shadow.appendChild(div(id));
  mk(live as any, shadow as any);
  assert.deepEqual(
    live.children.map((c: MockEl) => c.id),
    ["a", "x", "b", "y"],
  );
  assert.equal(live.children[0], a, "a reused");
  assert.equal(live.children[2], b, "b reused");
});

test("mk empties the live node when the shadow is empty", () => {
  const live = div();
  live.appendChild(div("d"));
  mk(live as any, div() as any);
  assert.equal(live.children.length, 0);
});

test("an unchanged __hk keeps the live node (listener preserved)", () => {
  const live = div();
  const btn = doc.createElement("button");
  btn.id = "k";
  btn.__hk = "p1_5_3,4";
  live.appendChild(btn);
  const shadow = div();
  const btn2 = doc.createElement("button");
  btn2.id = "k";
  btn2.__hk = "p1_5_3,4";
  shadow.appendChild(btn2);
  mk(live as any, shadow as any);
  assert.equal(live.children[0], btn, "same node object kept");
});

test("fm flushes a staged morph exactly once", () => {
  const live = div();
  live.appendChild(div("old"));
  const shadow = div();
  shadow.appendChild(div("new"));
  st.pm = { live: live as any, shadow: shadow as any };
  fm();
  assert.deepEqual(
    live.children.map((c: MockEl) => c.id),
    ["new"],
  );
  assert.equal(st.pm, null);
  fm(); // no-op
});

test("keyed reorder moves id-less nodes by identity (values travel)", () => {
  const live = div();
  const shadow = div();
  const mkInput = (k: number, v: string) => {
    const e = doc.createElement("input") as any;
    e.__k = k;
    e.value = v;
    return e;
  };
  const a = mkInput(1, "alpha"),
    b = mkInput(2, "beta"),
    c = mkInput(3, "gamma");
  for (const e of [a, b, c]) live.appendChild(e);
  // shadow: reordered [3, 1, 2], fresh nodes with empty values
  for (const k of [3, 1, 2]) shadow.appendChild(mkInput(k, ""));
  mk(live as any, shadow as any);
  const ks = live.children.map((e: any) => e.__k);
  assert.deepEqual(ks, [3, 1, 2], "order follows keys");
  assert.equal(live.children[0], c, "same node object moved");
  assert.equal(live.children[1], a);
  assert.equal(live.children[2], b);
  assert.deepEqual(
    live.children.map((e: any) => e.value),
    ["gamma", "alpha", "beta"],
    "uncontrolled input values travel with their items",
  );
});

test("keyed removal and insertion inside a reorder", () => {
  const live = div();
  const shadow = div();
  const item = (k: number) => {
    const e = doc.createElement("li") as any;
    e.__k = k;
    e.textContent = "k" + k;
    return e;
  };
  for (const k of [1, 2, 3]) live.appendChild(item(k));
  const kept2 = live.children[1];
  for (const k of [4, 3, 2]) shadow.appendChild(item(k));
  mk(live as any, shadow as any);
  assert.deepEqual(
    live.children.map((e: any) => e.__k),
    [4, 3, 2],
  );
  assert.equal(live.children[2], kept2, "surviving keyed node reused");
});

test("positional matching never consumes a keyed node", () => {
  const live = div();
  const keyed = doc.createElement("p") as any;
  keyed.__k = 9;
  keyed.textContent = "keyed";
  live.appendChild(keyed);
  const shadow = div();
  const plain = doc.createElement("p");
  plain.textContent = "plain";
  shadow.appendChild(plain);
  mk(live as any, shadow as any);
  assert.equal(live.children[0], plain, "unkeyed shadow gets a fresh node");
  assert.equal((live.children[0] as any).__k, undefined);
});

test("me syncs __k alongside __hk", () => {
  const a = div() as any,
    b = div() as any;
  b.__k = 42;
  me(a, b);
  assert.equal(a.__k, 42);
});

test("me syncs the checked property for checkbox inputs", () => {
  const live = doc.createElement("input") as any;
  live.setAttribute("type", "checkbox");
  live.setAttribute("checked", "");
  live.checked = true; // property, as the browser would have it
  const shadow = doc.createElement("input") as any;
  shadow.setAttribute("type", "checkbox"); // no checked attr: server says off
  me(live, shadow);
  assert.equal(live.getAttribute("checked"), null, "attribute synced");
  assert.equal(live.checked, false, "property synced too");
});
