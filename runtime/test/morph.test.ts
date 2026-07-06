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
