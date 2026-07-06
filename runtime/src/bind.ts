// BIND_LOCAL attaches like BIND_REMOTE in the JS runtime (a WASM build would
// dispatch locally); xi is that build's inline-handler parser, stubbed here.
// QUIRK preserved from the original: x() does `i = xi(d, i-1)` for IL/DH, so
// this stub would loop forever if those opcodes ever hit the wire — the
// server never emits them; do not "fix" without also handling stream skip.

import { V, type RwEl } from "./state.ts";
import { se, snd } from "./events.ts";

/** BIND_LOCAL: same server round-trip as BIND_REMOTE in the JS runtime. */
export function BL(f: number, t: number, h: number, r: RwEl[]): void {
  r[f].addEventListener(V[t] || "click", (e) => {
    e.preventDefault();
    snd(() => se(h, t, f, e, r[f]), e, r[f]);
  });
}

/** Inline-handler parser stub (WASM-only opcodes IL/DH). */
export function xi(_d: Uint8Array, i: number): number {
  return i;
}
