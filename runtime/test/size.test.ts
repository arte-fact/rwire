// Size budget: the artifact must stay within tolerance of the hand-minified
// original (12.6KB raw / 4.5KB gzipped after RT3 const-inlining). Raising these is a
// deliberate act — justify it in the commit message.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { gzipSync } from "node:zlib";

const MAX_RAW = 15_200;
const MAX_GZIP = 5_500;

test("runtime.min.js stays within the size budget", () => {
  let src: Buffer;
  try {
    src = readFileSync(new URL("../dist/runtime.min.js", import.meta.url));
  } catch {
    assert.fail("dist/runtime.min.js missing — run `npm run build` first");
  }
  const gz = gzipSync(src!).length;
  assert.ok(
    src!.length <= MAX_RAW,
    `minified ${src!.length} bytes exceeds budget ${MAX_RAW}`,
  );
  assert.ok(gz <= MAX_GZIP, `gzipped ${gz} bytes exceeds budget ${MAX_GZIP}`);
});
