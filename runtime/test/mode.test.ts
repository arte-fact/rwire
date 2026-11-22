import { test } from "node:test";
import assert from "node:assert/strict";

// `km` only touches `document.cookie`: a bare stand-in is enough.
const doc = { cookie: "" };
(globalThis as any).document = doc;
(globalThis as any).BASE = "";
const { km } = await import("../src/connect.ts");

test("the theme mode the page started in is not written; a change is kept in rw_mode", () => {
  km("system"); // first batch: the page's own mode
  assert.equal(doc.cookie, "", "the starting mode stays the app's (or the cookie's)");
  km("system");
  assert.equal(doc.cookie, "");
  km("dark"); // the user forced dark
  assert.equal(doc.cookie, "rw_mode=dark;path=/;max-age=31536000");
  doc.cookie = "";
  km("dark"); // unchanged: not rewritten
  assert.equal(doc.cookie, "");
  km(undefined); // a batch without the theme style
  assert.equal(doc.cookie, "");
  km("system"); // back to following the OS: kept too
  assert.equal(doc.cookie, "rw_mode=system;path=/;max-age=31536000");
});

test("under a base path the cookie is scoped to the app's mount", () => {
  // Two apps under different base paths must not share their mode.
  (globalThis as any).BASE = "/preview/x";
  doc.cookie = "";
  km("light"); // the previous test left "system": a change
  assert.equal(doc.cookie, "rw_mode=light;path=/preview/x;max-age=31536000");
  (globalThis as any).BASE = "";
});
