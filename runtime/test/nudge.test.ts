// Nudge extension: − / + buttons step an input by its step, clamped, and
// fire the events a hand on it would.
import { test } from "node:test";
import assert from "node:assert/strict";
import { makeDom } from "./dom.ts";
import { i as install } from "../src/ext/nudge.ts";

test("a nudge steps its input by n steps, clamped, and fires input then change", () => {
  const doc = makeDom().document as any;
  install(doc);
  const input = doc.createElement("input");
  input.setAttribute("id", "rate");
  Object.assign(input, { min: "0", max: "50", step: "5", value: "20" });
  const fired: string[] = [];
  input.dispatchEvent = (e: Event) => (fired.push(e.type), true);
  const minus = doc.createElement("button");
  minus.setAttribute("data-nudge", "-1");
  minus.setAttribute("data-nudge-for", "rate");
  const plus = doc.createElement("button");
  plus.setAttribute("data-nudge", "3");
  plus.setAttribute("data-nudge-for", "rate");
  const glyph = doc.createElement("span");
  plus.appendChild(glyph);
  for (const e of [input, minus, plus]) doc.body.appendChild(e);
  minus.click();
  assert.equal(input.value, "15");
  assert.deepEqual(fired, ["input", "change"]);
  glyph.click(); // a click inside the button counts
  assert.equal(input.value, "30");
  plus.click();
  assert.equal(input.value, "45");
  plus.click();
  assert.equal(input.value, "50", "clamped to max");
});

const field = (doc: any, attrs: Record<string, string>) => {
  const input = doc.createElement("input");
  input.setAttribute("id", "n");
  Object.assign(input, { min: "", max: "", step: "", ...attrs });
  input.dispatchEvent = () => true;
  doc.body.appendChild(input);
  return input;
};
const nudger = (doc: any, n: number, big?: number) => {
  const b = doc.createElement("button");
  b.setAttribute("data-nudge", String(n));
  b.setAttribute("data-nudge-for", "n");
  if (big) b.setAttribute("data-nudge-big", String(big));
  doc.body.appendChild(b);
  return b;
};

test("a nudge lands on a multiple of its move, and still reaches a bound off the grid", () => {
  const doc = makeDom().document as any;
  install(doc);
  const input = field(doc, { min: "1", max: "430", value: "37" });
  const plus = nudger(doc, 20),
    minus = nudger(doc, -20);
  plus.click();
  assert.equal(input.value, "40", "37 + a notch of 20 lands on 40");
  minus.click();
  minus.click();
  assert.equal(input.value, "1", "0 is below min: clamped to it");
  input.value = "420";
  plus.click();
  assert.equal(input.value, "430", "the max is reached although off the grid");
  minus.click();
  assert.equal(input.value, "420", "and from it back onto the grid");
});

test("held, a nudge repeats, then takes its big step on round counts; the closing click is not counted", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const doc = makeDom().document as any;
  install(doc);
  const input = field(doc, { min: "0", max: "1000", value: "0" });
  const plus = nudger(doc, 10, 50);
  plus.fire("pointerdown", { target: plus });
  t.mock.timers.tick(399);
  assert.equal(input.value, "0", "nothing before the wait");
  t.mock.timers.tick(1);
  assert.equal(input.value, "10");
  for (let k = 0; k < 5; k++) t.mock.timers.tick(110);
  assert.equal(input.value, "60", "six small steps");
  t.mock.timers.tick(110);
  assert.equal(input.value, "100", "then the big step, onto a multiple of 50");
  t.mock.timers.tick(110);
  assert.equal(input.value, "150");
  plus.fire("pointerup", { target: plus });
  plus.click();
  t.mock.timers.tick(1000);
  assert.equal(input.value, "150", "released: no more steps, and the click is swallowed");
  plus.click();
  assert.equal(input.value, "160", "the next tap counts again");
});
