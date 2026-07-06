import { test } from "node:test";
import assert from "node:assert/strict";
import { rv, wv } from "../src/varint.ts";

test("wv/rv round-trip across all width boundaries", () => {
  for (const v of [0, 1, 63, 127, 128, 129, 1000, 16511, 16512, 16513, 100000, 4210000]) {
    const a: number[] = [];
    wv(a, v);
    const [got, len] = rv(new Uint8Array(a), 0);
    assert.equal(got, v, `value ${v}`);
    assert.equal(len, a.length, `length of ${v}`);
  }
});

test("width selection matches the Rust encoder", () => {
  const width = (v: number) => {
    const a: number[] = [];
    wv(a, v);
    return a.length;
  };
  assert.equal(width(127), 1);
  assert.equal(width(128), 2);
  assert.equal(width(16511), 2);
  assert.equal(width(16512), 3);
});

test("rv reads at an offset and reports consumed bytes", () => {
  const a: number[] = [0xff, 0xff];
  wv(a, 300);
  const [v, len] = rv(new Uint8Array(a), 2);
  assert.equal(v, 300);
  assert.equal(len, 2);
});
