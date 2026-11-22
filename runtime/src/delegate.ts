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

import { st, type RwEl } from "./state.ts";
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
  const tap = type === "click" || type === "submit";
  while (n) {
    const recs = n.__b && n.__b[type];
    if (recs) {
      e.preventDefault();
      const c = tap ? tapped(e, n) : null;
      // A control still waiting on the server is not sent a second time.
      if (!c || !c.hasAttribute(BUSY)) {
        for (const r of recs) fire(r, e, n);
        if (c) busy(c);
      }
    }
    n = n.parentNode as RwEl | null;
  }
}

/** Fire `el`'s own `type` bindings only, without a DOM event and without
 *  bubbling: a control reporting to the server (a resize handle). */
export function own(el: RwEl, type: string): void {
  const recs = el.__b && el.__b[type];
  if (recs) for (const r of recs) fire(r, { type, target: el } as unknown as Event, el);
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

const BUSY = "data-busy";
let bt: ReturnType<typeof setTimeout> | undefined;

/** Release every busy control: the server answered every event, or it kept
 * silent too long (the net: a lost answer never freezes a control). */
export function release(): void {
  clearTimeout(bt);
  for (const c of document.querySelectorAll("[" + BUSY + "]")) {
    c.removeAttribute(BUSY);
    c.removeAttribute("aria-busy");
  }
}

/** EVENT_ACK: one event answered, its effects already applied; the busy
 * controls are released once every event sent is. */
export function ack(): void {
  if (st.pe > 0) st.pe--;
  if (!st.pe) release();
}

/** The control a tap presses: a submit's submit button, else the bound element. */
function tapped(e: Event, el: RwEl): Element | null {
  return e.type === "submit"
    ? (e as SubmitEvent).submitter || el.querySelector("button[type=submit],button:not([type])")
    : el;
}

/** Mark a tapped control busy, synchronously in the tap — before any network.
 * The attribute alone changes nothing visible: the stylesheet shows it only
 * after a moment, so a quick answer never flickers. A re-render of the control
 * by its answer drops it with the rest of the attributes the server did not set. */
function busy(c: Element): void {
  if (!st.pe) return; // nothing went out (socket closed)
  c.setAttribute(BUSY, "");
  c.setAttribute("aria-busy", "true");
  clearTimeout(bt);
  bt = setTimeout(() => {
    st.pe = 0;
    release();
  }, 10000);
}
