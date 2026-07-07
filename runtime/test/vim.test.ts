// M2 vim engine — table-driven: given text+caret, press keys, expect
// text/caret/selection/mode/fired-inputs. The module installs on the mock
// document exactly as the loader would.
import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { makeDom, type MockDoc, type MockEl } from "./dom.ts";
import { i as installVim } from "../src/ext/vim.ts";

let doc: MockDoc;
let ta: MockEl & { value: string; selectionStart: number; selectionEnd: number };
let inputs: number;

function press(...keys: string[]): void {
  for (const key of keys)
    for (const fn of doc.listeners["keydown"] || [])
      fn({ key, target: ta, preventDefault: () => {} });
}

beforeEach(() => {
  const dom = makeDom();
  doc = dom.document;
  (globalThis as any).document = doc;
  installVim(doc as any);
  ta = doc.createElement("textarea") as any;
  ta.setAttribute("data-vim", "normal");
  ta.value = "";
  ta.selectionStart = 0;
  ta.selectionEnd = 0;
  doc.body.appendChild(ta);
  inputs = 0;
  ta.addEventListener("input", () => inputs++);
});

const setup = (text: string, caret: number) => {
  ta.value = text;
  ta.setSelectionRange(caret, caret);
};

// -------------------------------------------------------------- motions
test("double install handles each keydown ONCE (browser side-effect + loader)", () => {
  installVim(doc as any); // second install: the real-browser scenario
  setup("foo bar", 0);
  press("v");
  assert.equal(ta.getAttribute("data-vim"), "v", "v must not self-cancel");
  press("w");
  assert.equal(ta.selectionStart, 0);
  assert.equal(ta.selectionEnd, 5, "single w extension (inclusive display)");
  press("d");
  assert.equal(ta.value, "ar", "vwd cuts once");
});

test("char-visual highlights exactly what operators cut", () => {
  setup("abcdef", 1);
  press("v");
  assert.equal(ta.selectionStart, 1);
  assert.equal(ta.selectionEnd, 2, "entering v selects the cursor char");
  press("l", "l");
  assert.equal(ta.selectionEnd, 4, "display inclusive of head");
  press("d");
  assert.equal(ta.value, "aef", "cut matches the highlight");
});

test("h l stay within the line", () => {
  setup("ab\ncd", 1);
  press("l"); assert.equal(ta.selectionStart, 2, "l stops at line end");
  press("h", "h"); assert.equal(ta.selectionStart, 0);
  press("h"); assert.equal(ta.selectionStart, 0, "h stops at line start");
});

test("j k move by line preserving column where possible", () => {
  setup("alpha\nhi\ngamma", 4);
  press("j"); assert.equal(ta.selectionStart, 8, "clamped to short line end");
  press("j"); assert.equal(ta.selectionStart, 11, "column 2 of gamma");
  press("k", "k"); assert.equal(ta.selectionStart, 2);
});

test("w b e word motions with punctuation classes", () => {
  setup("foo bar() baz", 0);
  press("w"); assert.equal(ta.selectionStart, 4, "w to bar");
  press("w"); assert.equal(ta.selectionStart, 7, "w to punct run");
  press("w"); assert.equal(ta.selectionStart, 10, "w to baz");
  press("b"); assert.equal(ta.selectionStart, 7);
  press("b"); assert.equal(ta.selectionStart, 4);
  press("e"); assert.equal(ta.selectionStart, 6, "e end of bar");
});

test("0 ^ $ line anchors", () => {
  setup("  hello", 5);
  press("0"); assert.equal(ta.selectionStart, 0);
  press("^"); assert.equal(ta.selectionStart, 2);
  press("$"); assert.equal(ta.selectionStart, 7);
});

test("gg G and counted G", () => {
  setup("one\ntwo\nthree", 5);
  press("g", "g"); assert.equal(ta.selectionStart, 0);
  press("G"); assert.equal(ta.selectionStart, 8, "G to last line first non-blank");
  press("2", "G"); assert.equal(ta.selectionStart, 4, "2G to line 2");
});

test("counts multiply motions", () => {
  setup("a b c d e", 0);
  press("3", "w"); assert.equal(ta.selectionStart, 6);
});

// ------------------------------------------------------------- operators
test("dw deletes to next word and fires input", () => {
  setup("foo bar baz", 0);
  press("d", "w");
  assert.equal(ta.value, "bar baz");
  assert.equal(ta.selectionStart, 0);
  assert.equal(inputs, 1, "one synthetic input");
});

test("d2w with count", () => {
  setup("foo bar baz", 0);
  press("d", "2", "w");
  assert.equal(ta.value, "baz");
});

test("de is inclusive; d$ to end of line", () => {
  setup("foo bar\nx", 0);
  press("d", "e");
  assert.equal(ta.value, " bar\nx");
  setup("foo bar\nx", 1);
  press("d", "$");
  assert.equal(ta.value, "f\nx");
});

test("dd removes the line; 2dd removes two", () => {
  setup("one\ntwo\nthree", 5);
  press("d", "d");
  assert.equal(ta.value, "one\nthree");
  setup("one\ntwo\nthree", 0);
  press("2", "d", "d");
  assert.equal(ta.value, "three");
});

test("yy p pastes linewise below; yw P pastes charwise before", () => {
  setup("one\ntwo", 0);
  press("y", "y");
  assert.equal(ta.value, "one\ntwo", "yank does not mutate");
  press("p");
  assert.equal(ta.value, "one\none\ntwo", "linewise paste below");
  setup("foo bar", 0);
  press("y", "w");
  press("P");
  assert.equal(ta.value, "foo foo bar");
});

test("cw acts like ce and enters insert", () => {
  setup("foo bar", 0);
  press("c", "w");
  assert.equal(ta.value, " bar", "word changed, space kept");
  assert.equal(ta.getAttribute("data-vim"), "insert");
});

test("cc clears the line into insert", () => {
  setup("one\ntwo\nthree", 5);
  press("c", "c");
  assert.equal(ta.value, "one\nthree");
  assert.equal(ta.getAttribute("data-vim"), "insert");
});

test("x deletes chars with count; D and C cut to line end", () => {
  setup("hello\nworld", 0);
  press("3", "x");
  assert.equal(ta.value, "lo\nworld");
  setup("hello\nworld", 2);
  press("D");
  assert.equal(ta.value, "he\nworld");
  setup("hello\nworld", 2);
  press("C");
  assert.equal(ta.value, "he\nworld");
  assert.equal(ta.getAttribute("data-vim"), "insert");
});

test("x then p re-pastes the register charwise", () => {
  setup("abc", 0);
  press("x");
  assert.equal(ta.value, "bc");
  press("p");
  assert.equal(ta.value, "bac", "pasted after caret");
});

// ---------------------------------------------------------- entering insert
test("i a I A o O enter insert at the right spots", () => {
  setup("  hi", 3);
  press("i"); assert.equal(ta.getAttribute("data-vim"), "insert");
  press("Escape");
  press("a"); assert.equal(ta.selectionStart, 4);
  press("Escape");
  press("I"); assert.equal(ta.selectionStart, 2);
  press("Escape");
  press("A"); assert.equal(ta.selectionStart, 4);
  press("Escape");
  setup("one\ntwo", 1);
  press("o");
  assert.equal(ta.value, "one\n\ntwo");
  assert.equal(ta.selectionStart, 4);
  press("Escape");
  setup("one\ntwo", 5);
  press("O");
  assert.equal(ta.value, "one\n\ntwo");
  assert.equal(ta.selectionStart, 4);
});

test("insert mode leaves typing alone; Escape returns to normal", () => {
  setup("hi", 0);
  press("i");
  let prevented = 0;
  for (const fn of doc.listeners["keydown"] || [])
    fn({ key: "x", target: ta, preventDefault: () => prevented++ });
  assert.equal(prevented, 0, "typing not intercepted in insert");
  press("Escape");
  assert.equal(ta.getAttribute("data-vim"), "normal");
});

// ----------------------------------------------------------------- visual
test("v extends with motions and d cuts the range", () => {
  setup("foo bar baz", 0);
  press("v", "2", "w");
  assert.equal(ta.selectionStart, 0);
  assert.equal(ta.selectionEnd, 9, "display inclusive of head char");
  press("d");
  assert.equal(ta.value, "az", "inclusive char range cut");
  assert.equal(ta.getAttribute("data-vim"), "normal");
});

test("V selects whole lines; Vd deletes them; Vy p duplicates", () => {
  setup("one\ntwo\nthree", 5);
  press("V");
  assert.equal(ta.selectionStart, 4);
  assert.equal(ta.selectionEnd, 8, "line two incl newline");
  press("j");
  assert.equal(ta.selectionEnd, 13, "extended to three");
  press("d");
  assert.equal(ta.value, "one");
  setup("one\ntwo", 0);
  press("V", "y", "p");
  assert.equal(ta.value, "one\none\ntwo");
});

test("Escape leaves visual without mutating", () => {
  setup("hello", 0);
  press("v", "l", "l", "Escape");
  assert.equal(ta.value, "hello");
  assert.equal(ta.getAttribute("data-vim"), "normal");
  assert.equal(ta.selectionStart, ta.selectionEnd);
});

// --------------------------------------------------------- undo delegation
test("u clicks the server undo; Ctrl-r the redo", () => {
  const undo = doc.createElement("span");
  undo.setAttribute("data-kbd", "mod+z");
  let undos = 0;
  undo.addEventListener("click", () => undos++);
  doc.body.appendChild(undo);
  const redo = doc.createElement("span");
  redo.setAttribute("data-kbd", "mod+shift+z");
  let redos = 0;
  redo.addEventListener("click", () => redos++);
  doc.body.appendChild(redo);
  setup("hi", 0);
  press("u");
  assert.equal(undos, 1);
  for (const fn of doc.listeners["keydown"] || [])
    fn({ key: "r", ctrlKey: true, target: ta, preventDefault: () => {} });
  assert.equal(redos, 1);
});

test("chip reflects every mode", () => {
  const chip = doc.createElement("span");
  chip.setAttribute("data-vim-chip", "1");
  doc.body.appendChild(chip);
  setup("hi", 0);
  press("v"); assert.equal(chip.textContent, "VISUAL");
  press("Escape"); assert.equal(chip.textContent, "NORMAL");
  press("V"); assert.equal(chip.textContent, "V-LINE");
  press("Escape", "i"); assert.equal(chip.textContent, "INSERT");
});

test("s substitutes: normal-mode chars, visual selection", () => {
  setup("word here", 0);
  press("s");
  assert.equal(ta.value, "ord here");
  assert.equal(ta.getAttribute("data-vim"), "insert");
  press("Escape");
  setup("foo bar", 0);
  press("v", "e", "s"); // vws-style flow: select then substitute
  assert.equal(ta.value, " bar", "selection substituted");
  assert.equal(ta.getAttribute("data-vim"), "insert");
});

// ------------------------------------------------------------- S1 objects
test("text objects: diw daw ciw viw", () => {
  setup("foo bar baz", 5);
  press("d", "i", "w");
  assert.equal(ta.value, "foo  baz", "diw removes the word only");
  setup("foo bar baz", 5);
  press("d", "a", "w");
  assert.equal(ta.value, "foo baz", "daw takes trailing space");
  setup("foo bar baz", 5);
  press("c", "i", "w");
  assert.equal(ta.value, "foo  baz");
  assert.equal(ta.getAttribute("data-vim"), "insert", "ciw enters insert");
  press("Escape");
  setup("foo bar baz", 5);
  press("v", "i", "w");
  assert.equal(ta.selectionStart, 4);
  assert.equal(ta.selectionEnd, 7, "viw selects the word");
});

test("text objects: quotes and brackets", () => {
  setup('say "hello there" end', 8);
  press("c", "i", '"');
  assert.equal(ta.value, 'say "" end', 'ci" empties the quotes');
  press("Escape");
  setup('say "hello there" end', 8);
  press("d", "a", '"');
  assert.equal(ta.value, "say  end", 'da" takes the quotes');
  setup("f(a, g(b)) tail", 5);
  press("d", "i", "(");
  assert.equal(ta.value, "f() tail", "di( from inside nested");
  setup("x = {a: 1} y", 6);
  press("c", "a", "{");
  assert.equal(ta.value, "x =  y", "ca{ takes the braces");
  press("Escape");
  setup("arr[idx] z", 5);
  press("d", "i", "[");
  assert.equal(ta.value, "arr[] z");
});

// ---------------------------------------------------------------- S2 seek
test("f t F T land the caret; ; and , repeat", () => {
  setup("abc.def.ghi", 0);
  press("f", ".");
  assert.equal(ta.selectionStart, 3);
  press(";");
  assert.equal(ta.selectionStart, 7, "; repeats forward");
  press(",");
  assert.equal(ta.selectionStart, 3, ", reverses");
  setup("abc.def", 0);
  press("t", ".");
  assert.equal(ta.selectionStart, 2, "t stops before");
  setup("abc.def", 6);
  press("F", ".");
  assert.equal(ta.selectionStart, 3);
  setup("abc.def", 6);
  press("T", ".");
  assert.equal(ta.selectionStart, 4);
});

test("df and dt compose; 2fx counts", () => {
  setup("one.two.three", 0);
  press("d", "f", ".");
  assert.equal(ta.value, "two.three", "df. inclusive");
  setup("one.two.three", 0);
  press("d", "t", ".");
  assert.equal(ta.value, ".two.three", "dt. exclusive of target");
  setup("a.b.c.d", 0);
  press("2", "f", ".");
  assert.equal(ta.selectionStart, 3, "2f. second occurrence");
});

test("W B E treat punctuation runs as one WORD", () => {
  setup("foo.bar() baz-qux end", 0);
  press("W");
  assert.equal(ta.selectionStart, 10, "W jumps the whole blob");
  press("W");
  assert.equal(ta.selectionStart, 18);
  press("B");
  assert.equal(ta.selectionStart, 10);
  setup("foo.bar baz", 0);
  press("E");
  assert.equal(ta.selectionStart, 6, "E end of WORD");
});

// ------------------------------------------------------------ S3 small ops
test("r replaces chars in place; 3rx repeats the char", () => {
  setup("hello", 0);
  press("r", "H");
  assert.equal(ta.value, "Hello");
  assert.equal(ta.getAttribute("data-vim"), "normal", "r stays normal");
  setup("hello", 0);
  press("3", "r", "x");
  assert.equal(ta.value, "xxxlo");
});

test("J joins lines with a single space, eating next indent", () => {
  setup("foo\n   bar\nbaz", 1);
  press("J");
  assert.equal(ta.value, "foo bar\nbaz");
  assert.equal(ta.selectionStart, 3, "caret at the join");
  setup("a\nb\nc\nd", 0);
  press("3", "J");
  assert.equal(ta.value, "a b c\nd", "3J joins three lines");
});

test("~ toggles case and advances", () => {
  setup("aBc", 0);
  press("~", "~", "~");
  assert.equal(ta.value, "AbC");
});

test(">> and << shift lines; visual > shifts the selection", () => {
  setup("one\ntwo", 0);
  press(">", ">");
  assert.equal(ta.value, "\tone\ntwo");
  press("<", "<");
  assert.equal(ta.value, "one\ntwo");
  setup("a\nb\nc", 0);
  press("2", ">", ">");
  assert.equal(ta.value, "\ta\n\tb\nc", "2>> shifts two lines");
  setup("a\nb\nc", 0);
  press("V", "j", ">");
  assert.equal(ta.value, "\ta\n\tb\nc", "visual > shifts selected lines once");
  assert.equal(ta.getAttribute("data-vim"), "normal", "exits visual");
});

test("% jumps between matching brackets and composes with d", () => {
  setup("f(a, (b))x", 1);
  press("%");
  assert.equal(ta.selectionStart, 8, "outer close");
  press("%");
  assert.equal(ta.selectionStart, 1, "back to open");
  setup("f(a, b) tail", 1);
  press("d", "%");
  assert.equal(ta.value, "f tail", "d% inclusive of both brackets");
});

test("{ and } move by paragraph and compose", () => {
  setup("a\nb\n\nc\nd\n\ne", 0);
  press("}");
  assert.equal(ta.selectionStart, 4, "next blank line");
  press("}");
  assert.equal(ta.selectionStart, 9);
  press("{");
  assert.equal(ta.selectionStart, 4);
  setup("a\nb\n\nc", 0);
  press("d", "}");
  assert.equal(ta.value, "\nc", "d} to the blank line");
});

test("Escape cancels pending operator/object/seek cleanly", () => {
  setup("hello world", 0);
  press("d", "Escape", "w");
  assert.equal(ta.value, "hello world", "canceled d does not fire on w");
  assert.equal(ta.selectionStart, 6, "w moved the caret normally");
  press("d", "i", "Escape");
  press("w");
  assert.equal(ta.value, "hello world", "canceled object too");
});

// ---------------------------------------------------------- S4 clipboard
test('"+yy mirrors the yank to the system clipboard', () => {
  let written = "";
  Object.defineProperty(globalThis, "navigator", { configurable: true, value: {
    clipboard: { writeText: (s: string) => ((written = s), Promise.resolve()) },
  } });
  setup("copy me\nrest", 0);
  press('"', "+", "y", "y");
  assert.equal(written, "copy me\n", "clipboard got the line");
  press("p");
  assert.equal(ta.value, "copy me\ncopy me\nrest", "unnamed register still set");
});

test('"+p inserts the clipboard text asynchronously', async () => {
  Object.defineProperty(globalThis, "navigator", { configurable: true, value: {
    clipboard: { readText: () => Promise.resolve("FROM-OS") },
  } });
  setup("ab", 0);
  press('"', "+", "p");
  await new Promise((r) => setTimeout(r, 0));
  assert.equal(ta.value, "aFROM-OSb", "pasted after the caret char");
});

test('"+p falls back to the unnamed register when the read is blocked', async () => {
  Object.defineProperty(globalThis, "navigator", { configurable: true, value: {
    clipboard: { readText: () => Promise.reject(new Error("blocked")) },
  } });
  setup("word x", 0);
  press("y", "w"); // unnamed = "word "
  press('"', "+", "p");
  await new Promise((r) => setTimeout(r, 0));
  assert.equal(ta.value, "wword ord x", "fallback pasted the unnamed register");
});

test("plain y/p never touch the clipboard", () => {
  let touched = false;
  Object.defineProperty(globalThis, "navigator", { configurable: true, value: {
    clipboard: { writeText: () => ((touched = true), Promise.resolve()) },
  } });
  setup("abc", 0);
  press("y", "w", "p");
  assert.equal(touched, false);
});
