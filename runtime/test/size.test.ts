// Size budget: the artifact must stay within tolerance of the hand-minified
// original (12.6KB raw / 4.5KB gzipped after RT3 const-inlining). Raising these is a
// deliberate act — justify it in the commit message.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { gzipSync } from "node:zlib";

// FROZEN (docs/vim-mode-design.md): features >~500B ship as lazy extensions.
// This line includes the extension CONTRACT's one-time core costs: the
// MOD_DEF loader and the client-owned-attribute morph rules (data-vim) —
// and, since the Empire merge, the live value bindings (live.ts, ~3KB raw /
// 1.3KB gzipped): they are wire opcodes the executor itself must parse
// (LIVE_SOURCE / LIVE_BIND), not a page feature an extension could add later.
// +500B raw / +200B gzip (2026-09-29): the tap's busy state and EVENT_ACK counting — it must
// run synchronously in the dispatcher on a page's first tap, so it cannot be lazy.
// +100B raw / +50B gzip (2026-09-30): on a reopen, the reload when /ready names another
// runtime version — a page left open across a deploy otherwise keeps its old extensions.
const MAX_RAW = 20_600;
const MAX_GZIP = 7_750;

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
