// Nudge extension — − / + buttons beside a slider or a number field.
//
// Lazy-loaded runtime extension: NOT part of the core capsule. The DOM
// contract: a click on (or inside) an element carrying `data-nudge="n"` and
// `data-nudge-for="<id>"` moves the input with that id by `n` of its `step`
// (1 without one; `n` may be negative), clamped to its `min`/`max`, then
// dispatches bubbling `input` and `change` events — exactly what a hand on
// the slider sends, so its live bindings follow and its bound handlers fire.
// Entirely client-side: no round-trip for the step itself.

/** Version stamp logged by the loader — bump with every change. */
export const v = 1;

const installed = new WeakSet<Document>();

type Field = HTMLInputElement;

/** Arm the one delegated click listener on `doc`. */
export function i(doc: Document): void {
  if (installed.has(doc)) return;
  installed.add(doc);
  doc.addEventListener("click", (ev: Event) => {
    let e = ev.target as Element | null;
    while (e && !(e.getAttribute && e.getAttribute("data-nudge"))) e = e.parentNode as Element | null;
    if (!e) return;
    const f = doc.getElementById(e.getAttribute("data-nudge-for") || "") as Field | null;
    if (!f) return;
    const n = +(e.getAttribute("data-nudge") || 0),
      to = +f.value + n * (+f.step || 1),
      lo = f.min === "" ? to : +f.min,
      hi = f.max === "" ? to : +f.max;
    f.value = String(Math.min(hi, Math.max(lo, to)));
    for (const t of ["input", "change"]) f.dispatchEvent(new Event(t, { bubbles: true }));
  });
}

if (typeof document !== "undefined") i(document);
