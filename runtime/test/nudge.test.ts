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
