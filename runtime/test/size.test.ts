// Size budget: the artifact must stay within tolerance of the hand-minified
// original (12.6KB raw / 4.5KB gzipped after RT3 const-inlining). Raising these is a
// deliberate act — justify it in the commit message.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { gzipSync } from "node:zlib";

// FROZEN (docs/vim-mode-design.md): features >~500B ship as lazy extensions.
// This line includes the extension CONTRACT's one-time core costs: the
// MOD_DEF loader and the client-owned-attribute morph rules (data-vim).
const MAX_RAW = 16_000;
const MAX_GZIP = 5_900;

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
