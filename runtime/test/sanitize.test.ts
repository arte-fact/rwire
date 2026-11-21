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

test("vbscript: and data: URLs refused in URL attrs", () => {
  for (const n of ["href", "src", "xlink:href", "formaction", "action"]) {
    for (const v of [
      "vbscript:msgbox(1)",
      " VBScript:x",
      "data:text/html,<script>alert(1)</script>",
      "DATA:text/html;base64,PHNjcmlwdD4=",
      "data:image/svg+xml,<svg onload=alert(1)>",
    ]) {
      const e = el();
      sa(e as any, n, v);
      assert.equal(e.getAttribute(n), null, `${n}=${v}`);
    }
  }
});

test("raster data: images allowed in src only", () => {
  const png = "data:image/png;base64,iVBORw0KGgo=";
  const e = el();
  sa(e as any, "src", png);
  assert.equal(e.getAttribute("src"), png);
  sa(e as any, "src", "data:image/JPEG,xx");
  assert.equal(e.getAttribute("src"), "data:image/JPEG,xx");
  // rwire-editor's previews carry no subtype.
  sa(e as any, "src", "data:image;base64,AAAA");
  assert.equal(e.getAttribute("src"), "data:image;base64,AAAA");
  const a = el();
  sa(a as any, "href", png);
  assert.equal(a.getAttribute("href"), null);
});

test("the app's own schemes pass through", () => {
  for (const v of ["magnet:?xt=urn:btih:x", "mailto:a@b.c", "tel:+331", "/rel", "#top"]) {
    const e = el();
    sa(e as any, "href", v);
    assert.equal(e.getAttribute("href"), v);
  }
});

test("javascript: text is fine in non-URL attrs", () => {
  const e = el();
  sa(e as any, "title", "javascript: the good parts");
  assert.equal(e.getAttribute("title"), "javascript: the good parts");
});

test("attribute names keep their case (SVG viewBox is case-sensitive)", () => {
  const e = el();
  sa(e as any, "viewBox", "0 0 24 24");
  assert.equal(e.getAttribute("viewBox"), "0 0 24 24");
  assert.equal(e.getAttribute("viewbox"), null);
});
