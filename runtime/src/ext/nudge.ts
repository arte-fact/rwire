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
//
// Whenever the input moves (a nudge or a hand), each of its nudges carries
// `aria-disabled="true"` while it stands on the bound it moves toward, and
// each `[data-nudge-say="<id>"]` reads the word of the mark nearest its
// value — `data-nudge-marks="50:le besoin|80:la ration pleine"`, within
// `data-nudge-near` (0: exact) — or else `data-nudge-rest`.

/** Version stamp logged by the loader — bump with every change. */
export const v = 3;

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

/** Tell the nudges and the says of the input `f` where it now stands. */
const follow = (doc: Document, f: Field): void => {
  const at = +f.value;
  doc.querySelectorAll(`[data-nudge-for="${f.id}"]`).forEach((b) => {
    const n = +(b.getAttribute("data-nudge") || 0),
      edge = n > 0 ? f.max !== "" && at >= +f.max : f.min !== "" && at <= +f.min;
    b.setAttribute("aria-disabled", String(edge));
  });
  doc.querySelectorAll(`[data-nudge-say="${f.id}"]`).forEach((s) => {
    const near = +(s.getAttribute("data-nudge-near") || 0);
    let word = s.getAttribute("data-nudge-rest") || "";
    for (const m of (s.getAttribute("data-nudge-marks") || "").split("|")) {
      const c = m.indexOf(":");
      if (c > 0 && Math.abs(+m.slice(0, c) - at) <= near) {
        word = m.slice(c + 1);
        break;
      }
    }
    if (s.textContent !== word) s.textContent = word;
    s.setAttribute("data-named", String(word !== (s.getAttribute("data-nudge-rest") || "")));
  });
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
  doc.addEventListener("input", (ev: Event) => {
    const f = ev.target as Field | null;
    if (f && f.id) follow(doc, f);
  });
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
