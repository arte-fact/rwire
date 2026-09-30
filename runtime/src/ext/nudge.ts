// Nudge extension — − / + buttons beside a slider or a number field.
//
// Lazy-loaded runtime extension: NOT part of the core capsule. The DOM
// contract: a click on (or inside) an element carrying `data-nudge="n"` and
// `data-nudge-for="<id>"` moves the input with that id by `n` of its `step`
// (1 without one; `n` may be negative), landing on a multiple of that move so
// the values stay round, clamped to its `min`/`max` (so a bound off the grid
// is still reached), then dispatches bubbling `input` and `change` events —
// exactly what a hand on the slider sends, so its live bindings follow and
// its bound handlers fire. Held down, the button repeats; after a few
// repeats it moves by `data-nudge-big` instead when it has one (the long
// step, landing on its own multiples). Entirely client-side: no round-trip
// for the step itself.

/** Version stamp logged by the loader — bump with every change. */
export const v = 2;

const installed = new WeakSet<Document>();

type Field = HTMLInputElement;

/** How long a press waits before repeating, the repeat's period, and how
 *  many small repeats come before the big step. */
const WAIT = 400,
  EVERY = 110,
  SMALL = 6;

const button = (ev: Event): Element | null => {
  let e = ev.target as Element | null;
  while (e && !(e.getAttribute && e.getAttribute("data-nudge"))) e = e.parentNode as Element | null;
  return e;
};

/** Move the input `e` names by `n` of its steps; false at a bound. */
const move = (doc: Document, e: Element, n: number): boolean => {
  const f = doc.getElementById(e.getAttribute("data-nudge-for") || "") as Field | null;
  if (!f || !n) return false;
  const s = Math.abs(n) * (+f.step || 1),
    at = +f.value,
    to = n > 0 ? Math.floor(at / s) * s + s : Math.ceil(at / s) * s - s,
    lo = f.min === "" ? to : +f.min,
    hi = f.max === "" ? to : +f.max,
    next = Math.min(hi, Math.max(lo, to));
  if (next === at) return false;
  f.value = String(next);
  for (const t of ["input", "change"]) f.dispatchEvent(new Event(t, { bubbles: true }));
  return true;
};

/** Arm the delegated listeners on `doc`. */
export function i(doc: Document): void {
  if (installed.has(doc)) return;
  installed.add(doc);
  let timer: ReturnType<typeof setTimeout> | undefined,
    held = false;
  const stop = () => clearTimeout(timer);
  doc.addEventListener("pointerdown", (ev: Event) => {
    const e = button(ev);
    if (!e) return;
    stop();
    held = false;
    let count = 0;
    const tick = () => {
      held = true;
      count++;
      const big = count > SMALL && e.getAttribute("data-nudge-big");
      if (move(doc, e, +(big || e.getAttribute("data-nudge") || 0))) timer = setTimeout(tick, EVERY);
    };
    timer = setTimeout(tick, WAIT);
  });
  for (const t of ["pointerup", "pointercancel"]) doc.addEventListener(t, stop);
  doc.addEventListener("click", (ev: Event) => {
    const e = button(ev);
    if (!e) return;
    // The click that ends a hold has already been counted.
    if (held) {
      held = false;
      return;
    }
    move(doc, e, +(e.getAttribute("data-nudge") || 0));
  });
}

if (typeof document !== "undefined") i(document);
