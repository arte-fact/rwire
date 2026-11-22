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
// +40B gzip (2026-11-21): the attribute sanitizer refuses `vbscript:` and `data:` URLs
// (images in `src` excepted) besides `javascript:`; it runs inside setAttribute, so it
// cannot be lazy.
// +200B raw / +80B gzip (theme foundations, R1): a theme mode the user picks survives a
// reload. After each batch the runtime keeps a changed `data-rw-mode` in the `rw_mode`
// cookie; it must see every batch, so it cannot be lazy.
// +300B raw / +100B gzip (shell navigation, R2): a resize handle can size the next
// sibling (a right-hand panel), reports the width to the server on release and resets it
// on double-click; it is the existing BIND_RESIZE opcode, so it cannot be lazy.
const MAX_RAW = 21_000;
const MAX_GZIP = 7_920;

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
