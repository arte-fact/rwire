import { test } from "node:test";
import assert from "node:assert/strict";
import { sa } from "../src/sanitize.ts";
import { makeDom } from "./dom.ts";

const el = () => makeDom().document.createElement("a");

test("plain attributes pass through", () => {
  const e = el();
  sa(e as any, "href", "https://example.com/x");
  assert.equal(e.getAttribute("href"), "https://example.com/x");
});

test("on* handlers are refused, any casing", () => {
  const e = el();
  sa(e as any, "onclick", "alert(1)");
  sa(e as any, "ONLOAD", "alert(1)");
  assert.equal(e.getAttribute("onclick"), null);
  assert.equal(e.getAttribute("onload"), null);
});

test("javascript: URLs refused in URL attrs, incl. control-char obfuscation", () => {
  for (const n of ["href", "src", "xlink:href", "formaction", "action"]) {
    const e = el();
    sa(e as any, n, "javascript:alert(1)");
    assert.equal(e.getAttribute(n), null, n);
    sa(e as any, n, "jav\tascri\npt:alert(1)".replace("pt", "pt")); // embedded controls
    assert.equal(e.getAttribute(n), null, `${n} obfuscated`);
    sa(e as any, n, " JAVASCRIPT:alert(1)");
    assert.equal(e.getAttribute(n), null, `${n} cased`);
  }
});

test("javascript: text is fine in non-URL attrs", () => {
  const e = el();
  sa(e as any, "title", "javascript: the good parts");
  assert.equal(e.getAttribute("title"), "javascript: the good parts");
});
