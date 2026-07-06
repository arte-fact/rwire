// Executor coverage: every opcode branch, driven by hand-built byte streams
// (the same encoding the Rust side produces; layouts from protocol/opcodes.rs).
import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { x } from "../src/executor.ts";
import { st, E, V, P, Y, AT, AV, SE, resetSession } from "../src/state.ts";
import { fl2, sl2, resetActions } from "../src/actions.ts";
import { rv, wv } from "../src/varint.ts";
import { makeDom, type MockEl } from "./dom.ts";

let doc: ReturnType<typeof makeDom>["document"];
let sent: Uint8Array[];
let closed: number;

// IntersectionObserver mock: captures instances; trigger() delivers entries.
class MockIO {
  static instances: MockIO[] = [];
  targets: unknown[] = [];
  disconnected = false;
  cb: (es: { isIntersecting: boolean }[]) => void;
  options: unknown;
  constructor(cb: (es: { isIntersecting: boolean }[]) => void, options: unknown) {
    this.cb = cb;
    this.options = options;
    MockIO.instances.push(this);
  }
  observe(t: unknown) {
    this.targets.push(t);
  }
  disconnect() {
    this.disconnected = true;
  }
  trigger(isIntersecting: boolean) {
    if (!this.disconnected) this.cb([{ isIntersecting }]);
  }
}

const clearMap = (m: Record<number, unknown>) => {
  for (const k of Object.keys(m)) delete m[k as unknown as number];
};

beforeEach(() => {
  doc = makeDom().document;
  (globalThis as any).document = doc;
  (globalThis as any).history = { pushState() {}, replaceState() {} };
  (globalThis as any).IntersectionObserver = MockIO;
  MockIO.instances = [];
  sent = [];
  closed = 0;
  st.w = { send: (m: Uint8Array) => sent.push(m), close: () => closed++ } as any;
  st.pm = null;
  st.DS = null;
  resetSession();
  resetActions();
  for (const m of [E, V, P, Y, AT, AV, SE]) clearMap(m);
});

// --- stream builders ---
const utf8 = (s: string) => [...new TextEncoder().encode(s)];
function syms(...strs: string[]): number[] {
  const b = [0xf0];
  wv(b, strs.length);
  for (const s of strs) {
    const u = utf8(s);
    wv(b, u.length);
    b.push(...u);
  }
  return b;
}
function mapDef(entries: [number, number, string][]): number[] {
  const b = [0x88];
  wv(b, entries.length);
  for (const [kind, code, name] of entries) {
    b.push(kind, code);
    const u = utf8(name);
    wv(b, u.length);
    b.push(...u);
  }
  return b;
}
const vint = (v: number) => {
  const b: number[] = [];
  wv(b, v);
  return b;
};
const run = (...parts: number[][]) => x(new Uint8Array(parts.flat().concat(0xff)));

const withErrors = (fn: () => void): string[] => {
  const errs: string[] = [];
  const orig = console.error;
  console.error = (...a: unknown[]) => errs.push(a.map(String).join(" "));
  try {
    fn();
  } finally {
    console.error = orig;
  }
  return errs;
};

// --- symbols, words, text ---

test("SYMBOLS + SET_TEXT via GET_BY_ID", () => {
  const target = doc.createElement("div");
  target.id = "greet";
  run(syms("greet", "hello"), [0x01, ...vint(0x80)], [0x11, 0, ...vint(0x81)]);
  assert.equal(target.textContent, "hello");
});

test("SYMBOLS_EXTEND appends at the given start index", () => {
  const target = doc.createElement("div");
  target.id = "t";
  run(syms("t"), [0x01, ...vint(0x80)]);
  // extend: 1 symbol starting at 0x81
  const ext = [0xf1, ...vint(1), ...vint(0x81)];
  const u = utf8("later");
  wv(ext, u.length);
  ext.push(...u);
  run(ext, [0x01, ...vint(0x80)], [0x11, 0, ...vint(0x81)]);
  assert.equal(target.textContent, "later");
});

test("WORD_TABLE + SET_TEXT_WORDS joins by spaces", () => {
  const t = doc.createElement("p");
  t.id = "w";
  const wt = [0xf2, 2];
  for (const wrd of ["hello", "world"]) {
    const u = utf8(wrd);
    wv(wt, u.length);
    wt.push(...u);
  }
  run(syms("w"), wt, [0x01, ...vint(0x80)], [0x13, 0, 2, 0, 1]);
  assert.equal(t.textContent, "hello world");
});

test("SET_TEXT_INT zigzag-decodes negative and positive", () => {
  const t = doc.createElement("span");
  t.id = "n";
  run(syms("n"), [0x01, ...vint(0x80)], [0x15, 0, ...vint(9)]); // zigzag 9 = -5
  assert.equal(t.textContent, "-5");
  run(syms("n"), [0x01, ...vint(0x80)], [0x15, 0, ...vint(10)]); // zigzag 10 = 5
  assert.equal(t.textContent, "5");
});

// --- element creation ---

test("CREATE uses MAP_DEF names; kind 6 marks SVG (createElementNS)", () => {
  run(
    mapDef([
      [0, 5, "span"],
      [6, 8, "svg"],
    ]),
    [0x02, 5],
    [0x02, 8],
    [0x20, ...vint(0xffff), ...vint(0)], // append r0 to body
    [0x20, ...vint(0xffff), ...vint(1)], // append r1 to body
  );
  assert.equal(doc.body.children[0].tagName, "SPAN");
  assert.equal(doc.body.children[1].tagName, "SVG");
  assert.ok((doc.body.children[1] as any).isSvg, "created via createElementNS");
  assert.equal(SE[8], 1);
});

test("CREATE falls back to div for unknown codes", () => {
  run([0x02, 42], [0x20, ...vint(0xffff), ...vint(0)]);
  assert.equal(doc.body.children[0].tagName, "DIV");
});

test("CREATE_SYNCED and GET_SYNCED address __synced_N spans; GS null-guards", () => {
  run([0x03, ...vint(7)], [0x20, ...vint(0xffff), ...vint(0)]);
  assert.equal(doc.body.children[0].id, "__synced_7");
  // GS on existing region targets it; on a missing one, ops become no-ops
  run(syms("x"), [0x05, ...vint(7)], [0x11, 0, ...vint(0x80)]);
  assert.equal(doc.body.children[0].textContent, "x");
  const errs = withErrors(() =>
    run(syms("y"), [0x05, ...vint(99)], [0x11, 0, ...vint(0x80)]),
  );
  assert.deepEqual(errs, [], "missing region must not error");
});

// --- attributes, class, style ---

test("SET_CLASS, SET_ATTR (A-map + symbols), SET_DATA", () => {
  const t = doc.createElement("div");
  t.id = "a";
  run(
    syms("a", "c1 c2", "newid", "key", "val"),
    [0x01, ...vint(0x80)],
    [0x10, 0, ...vint(0x81)], // class
    [0x12, 0, ...vint(4), ...vint(0x82)], // attr: A[4]='id'
    [0x14, 0, ...vint(0x83), ...vint(0x84)], // dataset
  );
  assert.equal(t.className, "c1 c2");
  assert.equal(t.getAttribute("id"), "newid");
  assert.equal(t.dataset.key, "val");
});

test("SET_ATTR_ENUM / BOOL / KEY_SYM use AT/AV maps", () => {
  const t = doc.createElement("input");
  t.id = "f";
  run(
    mapDef([
      [2, 1, "type"],
      [3, 2, "email"],
      [2, 3, "required"],
      [2, 4, "placeholder"],
    ]),
    syms("f", "Your email"),
    [0x01, ...vint(0x80)],
    [0x26, 0, 1, 2], // enum/enum
    [0x27, 0, 3], // bool
    [0x28, 0, 4, ...vint(0x81)], // key/symbol
  );
  assert.equal(t.getAttribute("type"), "email");
  assert.equal(t.getAttribute("required"), "");
  assert.equal(t.getAttribute("placeholder"), "Your email");
});

test("STYLE_SET cssText, STYLE_UTIL/MULTI classes, STYLE_PROP via P/Y maps", () => {
  const t = doc.createElement("div");
  t.id = "s";
  run(
    mapDef([
      [4, 1, "display"],
      [5, 2, "flex"],
    ]),
    syms("s", "color:red"),
    [0x01, ...vint(0x80)],
    [0x81, 0, ...vint(0x81)],
    [0x82, 0, ...vint(300)],
    [0x84, 0, 2, ...vint(7), ...vint(8)],
    [0x83, 0, 1, 2],
  );
  assert.equal(t.style.cssText, "color:red");
  for (const c of ["u300", "u7", "u8"]) assert.ok(t.classList.contains(c), c);
  assert.equal(t.style.display, "flex");
});

test("STYLE_PSEUDO and STYLE_BREAKPOINT add prefixed classes", () => {
  const t = doc.createElement("div");
  t.id = "p";
  run(
    syms("p"),
    [0x01, ...vint(0x80)],
    [0x89, 0, 2, 1, ...vint(50)],
    [0x8a, 0, 3, 1, ...vint(60)],
  );
  assert.ok(t.classList.contains("h2u50"));
  assert.ok(t.classList.contains("b3u60"));
});

test("COMPOSITE_TABLE registers ids; STYLE_COMPOSITE applies known and unknown", () => {
  const t = doc.createElement("div");
  t.id = "c";
  const ct = [0x86, ...vint(1), ...vint(4), 2, ...vint(10), ...vint(11)];
  run(syms("c"), ct, [0x01, ...vint(0x80)], [0x85, 0, ...vint(4)], [0x85, 0, ...vint(9)]);
  assert.ok(t.classList.contains("c4"));
  assert.ok(t.classList.contains("c9"), "unknown id still applies c-prefixed class");
});

test("STYLE_DEF inserts rules into the dedicated stylesheet", () => {
  const rule = ".u1{color:red}";
  const sd = [0x87, ...vint(1)];
  const u = utf8(rule);
  wv(sd, u.length);
  sd.push(...u);
  run(sd);
  assert.ok(st.DS, "style element created");
  assert.deepEqual(st.DS!.sheet!.cssRules, [rule]);
  assert.equal(doc.head.children[0], st.DS as unknown as MockEl);
});

// --- tree ops + morph staging ---

test("APPEND parents into refs and into body at the 0xFFFF sentinel", () => {
  run(
    mapDef([
      [0, 1, "ul"],
      [0, 2, "li"],
    ]),
    [0x02, 1],
    [0x02, 2],
    [0x20, ...vint(0), ...vint(1)],
    [0x20, ...vint(0xffff), ...vint(0)],
  );
  assert.equal(doc.body.children[0].tagName, "UL");
  assert.equal(doc.body.children[0].children[0].tagName, "LI");
});

test("CLEAR_CHILDREN stages a morph; BATCH_END flushes it (content replaced, node kept)", () => {
  const live = doc.createElement("div");
  live.id = "region";
  const old = doc.createElement("p");
  old.textContent = "old";
  live.appendChild(old);
  doc.body.appendChild(live);
  run(
    mapDef([[0, 1, "p"]]),
    syms("region", "new"),
    [0x01, ...vint(0x80)],
    [0x25, 0], // CLEAR_CHILDREN → r0 becomes the shadow
    [0x02, 1], // r1 = <p>
    [0x11, 1, ...vint(0x81)],
    [0x20, ...vint(0), ...vint(1)],
  );
  assert.equal(doc.body.children[0], live, "live node kept");
  assert.equal(live.children.length, 1);
  assert.equal(live.children[0].textContent, "new");
  assert.equal(live.children[0], old, "positional same-tag reuse morphs the old node");
});

// --- bindings ---

test("BIND_REMOTE sets __hk, listens, and a click sends the event", () => {
  const t = doc.createElement("button");
  t.id = "b";
  run(syms("b"), [0x01, ...vint(0x80)], [0x31, 0, 1, ...vint(9)]);
  assert.equal(t.__hk, "r1_9");
  t.fire("click");
  assert.equal(sent.length, 1);
  assert.equal(sent[0][0], 0);
});

test("BIND_LOCAL and BIND_REMOTE_PARAM register listeners; RP carries params", () => {
  const a = doc.createElement("button");
  a.id = "l";
  run(
    syms("l"),
    [0x01, ...vint(0x80)],
    [0x30, 0, 1, ...vint(3)],
    [0x34, 0, 1, ...vint(4), 2, 7, 7],
  );
  assert.equal(a.__hk, "p1_4_7,7");
  assert.equal(a.listeners["click"]?.length, 2);
  a.fire("click");
  assert.equal(sent.length, 2);
  assert.equal(sent[1][0], 0x80, "param variant marker");
});

test("BIND_DEBOUNCED sets a d-prefixed __hk and defers the send", () => {
  const t = doc.createElement("input");
  t.id = "d";
  run(syms("d"), [0x01, ...vint(0x80)], [0x33, 0, 1, ...vint(2), 0, 50]);
  assert.equal(t.__hk, "d1_2");
  t.fire("click");
  assert.equal(sent.length, 0, "debounced: nothing sent synchronously");
});

// --- routing ---

test("ROUTE_PUSH/REPLACE use symbols; INLINE variants carry the URL", () => {
  const calls: [string, string][] = [];
  (globalThis as any).history = {
    pushState: (_: unknown, __: string, u: string) => calls.push(["push", u]),
    replaceState: (_: unknown, __: string, u: string) => calls.push(["replace", u]),
  };
  const inline = (op: number, url: string) => {
    const b = [op];
    const u = utf8(url);
    wv(b, u.length);
    b.push(...u);
    return b;
  };
  run(
    syms("/a", "/b"),
    [0x70, ...vint(0x80)],
    [0x71, ...vint(0x81)],
    inline(0x72, "/c/9"),
    inline(0x73, "/d/9"),
  );
  assert.deepEqual(calls, [
    ["push", "/a"],
    ["replace", "/b"],
    ["push", "/c/9"],
    ["replace", "/d/9"],
  ]);
});

// --- client actions ---

test("INIT_TARGET + BIND_TARGET + BIND_TOGGLE flip classes through state", () => {
  const t = doc.createElement("div");
  t.id = "m";
  const btn = doc.createElement("button");
  btn.id = "k";
  run(
    syms("m", "k"),
    [0x47, 3, 0], // target 3 = false
    [0x01, ...vint(0x80)],
    [0x48, ...vint(0), 3, ...vint(77), 0], // bind: class u77 when true
    [0x01, ...vint(0x81)],
    [0x49, ...vint(1), 1, 3], // toggle on click
  );
  assert.equal(fl2[3], false);
  assert.ok(!t.classList.contains("u77"));
  btn.fire("click");
  assert.equal(fl2[3], true);
  assert.ok(t.classList.contains("u77"));
  btn.fire("click");
  assert.ok(!t.classList.contains("u77"));
});

test("inverted BIND_TARGET applies the class while false", () => {
  const t = doc.createElement("div");
  t.id = "inv";
  run(syms("inv"), [0x47, 1, 0], [0x01, ...vint(0x80)], [0x48, ...vint(0), 1, ...vint(5), 1]);
  assert.ok(t.classList.contains("u5"), "inverted binding active at false");
});

test("INIT_SELECTOR + BIND_SELECTOR + BIND_SELECT switch exclusive classes", () => {
  const a = doc.createElement("div");
  a.id = "ta";
  const b = doc.createElement("div");
  b.id = "tb";
  const btn = doc.createElement("button");
  btn.id = "sw";
  run(
    syms("ta", "tb", "sw"),
    [0x4a, 2, 0], // selector 2 = variant 0
    [0x01, ...vint(0x80)],
    [0x4b, ...vint(0), 2, 0, ...vint(11)], // a: u11 when variant 0
    [0x01, ...vint(0x81)],
    [0x4b, ...vint(1), 2, 1, ...vint(12)], // b: u12 when variant 1
    [0x01, ...vint(0x82)],
    [0x4c, ...vint(2), 1, 2, 1], // click → variant 1
  );
  assert.ok(a.classList.contains("u11"));
  assert.ok(!b.classList.contains("u12"));
  btn.fire("click");
  assert.equal(sl2[2], 1);
  assert.ok(!a.classList.contains("u11"));
  assert.ok(b.classList.contains("u12"));
});

test("BIND_TIMED_TOGGLE sets true immediately (revert is timer-based)", () => {
  const t = doc.createElement("div");
  t.id = "tt";
  const btn = doc.createElement("button");
  btn.id = "go";
  run(
    syms("tt", "go"),
    [0x47, 5, 0],
    [0x01, ...vint(0x80)],
    [0x48, ...vint(0), 5, ...vint(20), 0],
    [0x01, ...vint(0x81)],
    [0x4d, ...vint(1), 1, 5, 0, 10],
  );
  btn.fire("click");
  assert.equal(fl2[5], true);
  assert.ok(t.classList.contains("u20"));
});

test("AUTO_TOGGLE consumes its bytes and schedules the flip", () => {
  const errs = withErrors(() => run([0x4e, 1, 0, 5]));
  assert.deepEqual(errs, []);
});

// --- scroll sentinel ---

test("BIND_SENTINEL observes, fires once with params, then disconnects", () => {
  const s = doc.createElement("div");
  s.id = "sent";
  run(syms("sent"), [0x01, ...vint(0x80)], [0x4f, ...vint(0), ...vint(9), 1, 3]);
  assert.equal(s.__hk, "v9_3");
  assert.equal(MockIO.instances.length, 1);
  const ob = MockIO.instances[0];
  assert.equal(ob.targets[0], s);
  assert.match(String((ob.options as any).rootMargin), /%/);

  ob.trigger(false); // not intersecting: nothing
  assert.equal(sent.length, 0);

  ob.trigger(true);
  assert.equal(sent.length, 1);
  assert.ok(ob.disconnected, "one-shot: disconnected after fire");
  const m = sent[0];
  assert.equal(m[0], 0x80, "param-variant marker");
  let i = 1;
  const [h, hl] = rv(m, i);
  i += hl;
  assert.equal(h, 9);
  assert.equal(m[i++], 0x0e, "Ev::Visible byte");
  assert.equal(m[i++], 0, "ref");
  assert.equal(m[i++], 1, "param length");
  assert.equal(m[i++], 3, "next-chunk param");

  ob.trigger(true); // after disconnect: no double fire
  assert.equal(sent.length, 1);
});

test("BIND_SENTINEL with empty params still fires and keys the binding", () => {
  const s = doc.createElement("div");
  s.id = "s0";
  run(syms("s0"), [0x01, ...vint(0x80)], [0x4f, ...vint(0), ...vint(2), 0]);
  assert.equal(s.__hk, "v2_");
  MockIO.instances[0].trigger(true);
  assert.equal(sent.length, 1);
});

// --- error paths + batch end ---

test("unknown opcode reports and continues to later ops", () => {
  const t = doc.createElement("div");
  t.id = "after";
  const errs = withErrors(() =>
    run([0x99], syms("after", "still works"), [0x01, ...vint(0x80)], [0x11, 0, ...vint(0x81)]),
  );
  assert.equal(errs.length, 1);
  assert.match(errs[0], /^Unknown opcode 0x99 at pos 0/);
  assert.equal(t.textContent, "still works");
});

test("a truncated stream reports PARSE ERROR and closes the socket", () => {
  const errs = withErrors(() => x(new Uint8Array([0x11, 0]))); // SET_TEXT missing its symbol
  assert.ok(errs.some((e) => e.startsWith("PARSE ERROR")), errs.join("\n"));
  assert.equal(closed, 1);
});

test("BATCH_END restores focus context without throwing", () => {
  const input = doc.createElement("input");
  input.id = "q";
  input.value = "abc";
  doc.activeElement = input;
  const errs = withErrors(() => run(syms("q"), [0x01, ...vint(0x80)]));
  assert.deepEqual(errs, []);
});

// --- keyed morphing (T1) ---

test("SET_KEY stores the __k expando", () => {
  const t = doc.createElement("li");
  t.id = "kk";
  run(syms("kk"), [0x01, ...vint(0x80)], [0x16, 0, ...vint(300)]);
  assert.equal(t.__k, 300);
});

test("BATCH_END restores selection on an id-less surviving element", () => {
  const input = doc.createElement("input");
  doc.body.appendChild(input);
  input.value = "abc";
  doc.activeElement = input;
  let restored: [number, number] | null = null;
  input.setSelectionRange = (a: number, b: number) => {
    restored = [a, b];
  };
  (input as any).selectionStart = 1;
  (input as any).selectionEnd = 2;
  run([]);
  assert.deepEqual(restored, [1, 2], "selection restored via the node object");
});
