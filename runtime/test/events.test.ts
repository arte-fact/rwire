import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { gp, se, sep } from "../src/events.ts";
import { st } from "../src/state.ts";
import { rv } from "../src/varint.ts";
import { makeDom } from "./dom.ts";

let doc: ReturnType<typeof makeDom>["document"];
let sent: Uint8Array[];
beforeEach(() => {
  doc = makeDom().document;
  sent = [];
  st.w = { send: (m: Uint8Array) => sent.push(m), close() {} } as any;
});

const fakeEv = (type: string, target?: unknown) => {
  let prevented = false;
  return {
    type,
    target,
    preventDefault: () => (prevented = true),
    wasPrevented: () => prevented,
  };
};

test("gp collects form fields as JSON on submit and prevents default", () => {
  const form = doc.createElement("form");
  (globalThis as any).FormData = class {
    constructor(_f: unknown) {}
    forEach(cb: (v: string, k: string) => void) {
      cb("alice", "name");
      cb("42", "age");
    }
  };
  const e = fakeEv("submit", form);
  const p = gp(e as any, form as any);
  assert.deepEqual(JSON.parse(p), { t: "form", v: { name: "alice", age: "42" } });
  assert.ok(e.wasPrevented());
});

test("gp collects control value on input/change", () => {
  const ta = doc.createElement("textarea");
  ta.value = "draft";
  const p = gp(fakeEv("input", ta) as any, ta as any);
  assert.deepEqual(JSON.parse(p), { t: "text", v: "draft" });
});

test("gp collects data-* on click, empty when none", () => {
  const el = doc.createElement("button");
  el.dataset.filter = "done";
  const p = gp(fakeEv("click", el) as any, el as any);
  assert.deepEqual(JSON.parse(p), { t: "data", v: { filter: "done" } });

  const bare = doc.createElement("button");
  assert.equal(gp(fakeEv("click", bare) as any, bare as any), "");
});

test("se wire layout: [0, handler, type, ref&255, len, payload]", () => {
  const el = doc.createElement("button");
  se(300, 1, 7, fakeEv("click", el) as any, el as any);
  const m = sent[0];
  assert.equal(m[0], 0);
  let i = 1;
  const [h, hl] = rv(m, i);
  i += hl;
  assert.equal(h, 300);
  assert.equal(m[i++], 1); // event type
  assert.equal(m[i++], 7); // ref & 255
  const [len, ll] = rv(m, i);
  i += ll;
  assert.equal(len, 0);
  assert.equal(i, m.length);
});

test("sep wire layout carries the param bytes after 0x80 marker", () => {
  const el = doc.createElement("li");
  const prm = new Uint8Array([9, 9, 3]);
  sep(5, 1, 2, prm, fakeEv("click", el) as any, el as any);
  const m = sent[0];
  assert.equal(m[0], 0x80);
  let i = 1;
  const [h, hl] = rv(m, i);
  i += hl;
  assert.equal(h, 5);
  assert.equal(m[i++], 1);
  assert.equal(m[i++], 2);
  assert.equal(m[i++], 3); // param length
  assert.deepEqual([...m.slice(i, i + 3)], [9, 9, 3]);
  i += 3;
  const [len, ll] = rv(m, i);
  i += ll;
  assert.equal(len, 0);
  assert.equal(i, m.length);
});
