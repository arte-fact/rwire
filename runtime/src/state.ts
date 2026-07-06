// Shared mutable runtime state. The original hand-minified runtime used bare
// globals (`s`, `wt`, `w`, …); ESM forbids cross-module reassignment of
// exported `let`, so the reassignable ones live as properties of one `st`
// object. Name maps are mutated in place (never reassigned) and stay separate.

/// An element in the executor's ref array. Expandos: `__hk` is the binding key
/// the morph uses to decide listener reuse; `__t` is the input-debounce timer.
export type RwEl = HTMLElement & {
  __hk?: string;
  __t?: ReturnType<typeof setTimeout>;
};

export interface PendingMorph {
  live: Element;
  shadow: Element;
}

export const st = {
  /** Symbol table: index → string (session range starts at 0x80). */
  s: {} as Record<number, string>,
  /** Word table for SET_TEXT_WORDS. */
  wt: [] as string[],
  /** The WebSocket; set by connect() before anything can use it. */
  w: null as WebSocket | null,
  /** Next symbol slot during SYMBOLS/SYMBOLS_EXTEND parsing. */
  sc: 0,
  /** Composite id → class name (COMPOSITE_TABLE). */
  K: {} as Record<number, string>,
  /** The lazily created <style> for STYLE_DEF rules. */
  DS: null as HTMLStyleElement | null,
  /** Morph staged by CLEAR_CHILDREN, flushed at fm(). */
  pm: null as PendingMorph | null,
};

// Name maps: (code → name), delivered lazily over the wire via MAP_DEF and
// only ever grown — a reconnect resets symbols/words but keeps these, exactly
// like the original capsule-level `const` maps.
export const E: Record<number, string> = {}; // elements
export const V: Record<number, string> = {}; // events
export const P: Record<number, string> = {}; // style props
export const Y: Record<number, string> = {}; // style values
export const AT: Record<number, string> = {}; // attr keys
export const AV: Record<number, string> = {}; // attr values
export const SE: Record<number, 1> = {}; // SVG element codes (createElementNS)

/** Attr-symbol fallback map; 0x04 = "id" is the one universally seeded entry. */
export const A: Record<number, string> = { 4: "id" };

/** Reset per-connection state on reconnect (name maps intentionally survive). */
export function resetSession(): void {
  st.s = {};
  st.wt = [];
  st.K = {};
  st.sc = 0;
}
