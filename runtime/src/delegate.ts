// Event delegation (T3): remote/local/debounced/param bindings store a small
// record on the element (`__b`) instead of attaching a listener; ONE
// document-level dispatcher per event type walks from the event target up the
// tree, firing every element's records — bubbling semantics preserved, and a
// 10,000-row list costs 10,000 records + one listener instead of 10,000
// closures. Morphs sync `__b` from the shadow, so reused nodes always carry
// the LATEST binding (fresh handler ids by construction).
//
// Client actions, sentinels, and resize handles keep direct listeners: they
// are rare, and two of them aren't DOM events at all.

import { type RwEl } from "./state.ts";
import { se, sep, snd } from "./events.ts";

/** One binding: handler id, bind-time ref (diagnostic byte on the wire),
 * optional debounce ms (BIND_DEBOUNCED) with its timer, optional params. */
export interface Binding {
  h: number;
  t: number;
  f: number;
  ms?: number;
  tm?: ReturnType<typeof setTimeout>;
  prm?: Uint8Array;
}

const installed = new Set<string>();

/** Test hook: forget installed dispatchers (tests swap the document; a real
 * page never does). */
export function resetDelegation(): void {
  installed.clear();
}

/** Register a binding and lazily install the type's document dispatcher. */
export function bind(el: RwEl, type: string, rec: Binding): void {
  const map = (el.__b ||= {});
  (map[type] ||= []).push(rec);
  if (!installed.has(type)) {
    installed.add(type);
    // focus/blur don't bubble; capture phase sees them anywhere in the tree.
    const capture = type === "focus" || type === "blur";
    document.addEventListener(type, (e) => dispatch(type, e), capture);
  }
}

function dispatch(type: string, e: Event): void {
  let n = e.target as RwEl | null;
  while (n) {
    const recs = n.__b && n.__b[type];
    if (recs) {
      e.preventDefault();
      for (const r of recs) fire(r, e, n);
    }
    n = n.parentNode as RwEl | null;
  }
}

function fire(r: Binding, e: Event, el: RwEl): void {
  if (r.ms !== undefined) {
    clearTimeout(r.tm);
    r.tm = setTimeout(() => se(r.h, r.t, r.f, e, el), r.ms);
  } else if (r.prm) {
    snd(() => sep(r.h, r.t, r.f, r.prm!, e, el), e, el);
  } else {
    snd(() => se(r.h, r.t, r.f, e, el), e, el);
  }
}
