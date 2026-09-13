// Client actions: Targets (bool toggle) and Selectors (exclusive enum) —
// zero-latency class flips entirely in the browser (action.rs is the server
// half). The original capsule included this only when the app used actions
// (`has_client_actions`); the bundle always carries it (~250 bytes) so the
// executor branches need no typeof guards.

interface TargetBinding {
  e: Element;
  s: number; // St utility code
  n: boolean; // invert
}

interface SelectorBinding {
  e: Element;
  v: number; // match value
  s: number; // St utility code
}

export let fl2: Record<number, boolean> = {}; // target states
export let fb2: Record<number, TargetBinding[]> = {}; // target bindings
export let sl2: Record<number, number> = {}; // selector states
export let sb2: Record<number, SelectorBinding[]> = {}; // selector bindings

/** Re-apply every binding of target `i` to its current bool state. */
export function uf2(i: number): void {
  const v = fl2[i];
  for (const b of fb2[i] || []) {
    if (v !== b.n) b.e.classList.add("u" + b.s);
    else b.e.classList.remove("u" + b.s);
  }
}

/** Re-apply every binding of selector `i` to its current value. */
export function us2(i: number): void {
  const v = sl2[i];
  for (const b of sb2[i] || []) {
    if (v === b.v) b.e.classList.add("u" + b.s);
    else b.e.classList.remove("u" + b.s);
  }
}

/** Reset all action state on reconnect (bindings point at removed DOM). */
export function resetActions(): void {
  fl2 = {};
  fb2 = {};
  sl2 = {};
  sb2 = {};
}
