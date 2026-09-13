import { test } from "node:test";
import assert from "node:assert/strict";
import { bx, bj } from "../src/connect.ts";

test("bx strips the mount prefix, bj joins it", () => {
  (globalThis as any).BASE = "/app";
  assert.equal(bx("/app/users/1"), "/users/1");
  assert.equal(bx("/app"), "/");
  assert.equal(bx("/other"), "/other");
  assert.equal(bj("/users/1"), "/app/users/1");
});

test("empty BASE is a passthrough", () => {
  (globalThis as any).BASE = "";
  assert.equal(bx("/users/1"), "/users/1");
  assert.equal(bj("/users/1"), "/users/1");
});
