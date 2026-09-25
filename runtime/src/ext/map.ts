// Map extension — pan and zoom of an SVG by pointer, wheel and pinch.
//
// Lazy-loaded runtime extension: NOT part of the core capsule. The DOM
// contract: an `<svg data-map viewBox="x y w h">` (rendered with the default
// `preserveAspectRatio`, xMidYMid meet) is dragged with one pointer, pinched
// with two and zoomed with the wheel; the extension writes the `viewBox`
// itself, then, once the gesture is over, copies it into the value of the
// `<input data-map-view>` beside the svg and dispatches a bubbling `input`
// event, so the server's bound handler learns where the viewer looks and
// its next render agrees with the screen. `data-map-extent="x y w h"` (the
// whole map) bounds the zoom: no wider than one and a half times it, no
// narrower than a sixty-fourth. A drag swallows the click that would follow.
// Any `[data-map-at="x y"]` element in the svg's container is pinned at that
// point of the map (its `left`/`top` in px), after every drag, zoom, patch
// or resize: a popover anchored to a region follows it.

/** Version stamp logged by the loader — bump with every change. */
export const v = 2;

type Box = [number, number, number, number];
interface Gesture {
  /** Pointers down, by id: their last screen position. */
  ptrs: Map<number, [number, number]>;
  /** Distance between the two pointers at the last pinch step. */
  pinch: number;
  /** Whether this gesture moved the map (then its click is swallowed). */
  moved: boolean;
  /** The debounce of the report to the server. */
  t: ReturnType<typeof setTimeout> | null;
}

const G = new WeakMap<SVGSVGElement, Gesture>();
const installed = new WeakSet<Document>();
/** The svg each pointer went down on: a drag is followed wherever the
 * pointer goes, and let go wherever it comes up, without capturing it (a
 * captured pointer's click would land on the svg, not on the region). */
const active = new Map<number, SVGSVGElement>();

const box = (el: SVGSVGElement): Box =>
  (el.getAttribute("viewBox") || "0 0 1 1").trim().split(/[\s,]+/).map(Number) as Box;
const extent = (el: SVGSVGElement): Box | null => {
  const e = el.getAttribute("data-map-extent");
  return e ? (e.trim().split(/[\s,]+/).map(Number) as Box) : null;
};

function gesture(el: SVGSVGElement): Gesture {
  let g = G.get(el);
  if (!g) {
    g = { ptrs: new Map(), pinch: 0, moved: false, t: null };
    G.set(el, g);
  }
  return g;
}

/** Map units per screen pixel and the padding of the centred drawing
 * (`preserveAspectRatio` meet). */
function scale(el: SVGSVGElement, b: Box): [number, number, number] {
  const r = el.getBoundingClientRect();
  const s = Math.max(b[2] / (r.width || 1), b[3] / (r.height || 1));
  return [s, r.left + (r.width - b[2] / s) / 2, r.top + (r.height - b[3] / s) / 2];
}

/** Pin every `[data-map-at="x y"]` element of the map's container at that
 * point of the map (map units), as `left`/`top` in px of the container: a
 * popover anchored to a region follows the drag and the zoom. */
function place(el: SVGSVGElement): void {
  const c = el.parentElement;
  if (!c) return;
  const b = box(el);
  const [s, ox, oy] = scale(el, b);
  const cr = c.getBoundingClientRect();
  c.querySelectorAll("[data-map-at]").forEach((n) => {
    const at = (n.getAttribute("data-map-at") || "").trim().split(/[\s,]+/).map(Number);
    if (at.length < 2 || at.some((v) => !isFinite(v))) return;
    const h = n as HTMLElement;
    h.style.left = `${ox + (at[0] - b[0]) / s - cr.left}px`;
    h.style.top = `${oy + (at[1] - b[1]) / s - cr.top}px`;
  });
}

/** Every map of the document, pinned afresh: after a patch, a resize. */
function placeAll(doc: Document): void {
  doc.querySelectorAll("svg[data-map]").forEach((el) => place(el as SVGSVGElement));
}

function write(el: SVGSVGElement, b: Box): void {
  el.setAttribute("viewBox", b.map((n) => Math.round(n)).join(" "));
  place(el);
  const g = gesture(el);
  if (g.t) clearTimeout(g.t);
  g.t = setTimeout(() => {
    g.t = null;
    const input = (el.parentElement || el.ownerDocument).querySelector(
      "input[data-map-view]",
    ) as HTMLInputElement | null;
    if (!input) return;
    input.value = el.getAttribute("viewBox") || "";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }, 150);
}

/** Zoom by `f` (< 1 closer) about the screen point (`px`, `py`). */
function zoom(el: SVGSVGElement, f: number, px: number, py: number): void {
  const b = box(el);
  const e = extent(el);
  const w = b[2] * f;
  if (e && (w > e[2] * 1.5 || w < e[2] / 64)) return;
  const [s, ox, oy] = scale(el, b);
  const mx = b[0] + (px - ox) * s;
  const my = b[1] + (py - oy) * s;
  write(el, [mx - (mx - b[0]) * f, my - (my - b[1]) * f, w, b[3] * f]);
}

function pan(el: SVGSVGElement, dx: number, dy: number): void {
  const b = box(el);
  const [s] = scale(el, b);
  write(el, [b[0] - dx * s, b[1] - dy * s, b[2], b[3]]);
}

const target = (e: Event): SVGSVGElement | null => {
  const t = e.target as Element | null;
  return t && t.closest ? (t.closest("svg[data-map]") as SVGSVGElement | null) : null;
};

export function i(doc: Document): void {
  if (installed.has(doc)) return;
  installed.add(doc);
  // Anchored elements: pinned now, after every patch of the page, on resize.
  placeAll(doc);
  let pending = 0;
  const later = () => {
    if (pending) return;
    pending = requestAnimationFrame(() => {
      pending = 0;
      placeAll(doc);
    });
  };
  if (typeof MutationObserver !== "undefined")
    new MutationObserver(later).observe(doc.body || doc, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ["data-map-at", "viewBox"],
    });
  doc.defaultView && doc.defaultView.addEventListener("resize", later);
  doc.addEventListener(
    "wheel",
    (e) => {
      const el = target(e);
      if (!el) return;
      e.preventDefault();
      const we = e as WheelEvent;
      zoom(el, we.deltaY > 0 ? 1.25 : 0.8, we.clientX, we.clientY);
    },
    { passive: false },
  );
  doc.addEventListener("pointerdown", (e) => {
    const el = target(e);
    if (!el) return;
    const pe = e as PointerEvent;
    const g = gesture(el);
    if (g.ptrs.size === 0) g.moved = false;
    g.ptrs.set(pe.pointerId, [pe.clientX, pe.clientY]);
    g.pinch = 0;
    active.set(pe.pointerId, el);
  });
  doc.addEventListener("pointermove", (e) => {
    const pe = e as PointerEvent;
    const el = active.get(pe.pointerId);
    if (!el) return;
    const g = gesture(el);
    const was = g.ptrs.get(pe.pointerId);
    if (!was) return;
    g.ptrs.set(pe.pointerId, [pe.clientX, pe.clientY]);
    if (g.ptrs.size === 1) {
      const dx = pe.clientX - was[0];
      const dy = pe.clientY - was[1];
      if (!g.moved && Math.abs(dx) + Math.abs(dy) < 4) {
        g.ptrs.set(pe.pointerId, was);
        return;
      }
      g.moved = true;
      pan(el, dx, dy);
    } else if (g.ptrs.size === 2) {
      const [a, b] = [...g.ptrs.values()];
      const d = Math.hypot(a[0] - b[0], a[1] - b[1]);
      if (g.pinch > 0 && d > 0) {
        g.moved = true;
        zoom(el, g.pinch / d, (a[0] + b[0]) / 2, (a[1] + b[1]) / 2);
      }
      g.pinch = d;
    }
  });
  const up = (e: Event) => {
    const id = (e as PointerEvent).pointerId;
    const el = active.get(id);
    if (!el) return;
    active.delete(id);
    const g = gesture(el);
    g.ptrs.delete(id);
    g.pinch = 0;
  };
  doc.addEventListener("pointerup", up);
  doc.addEventListener("pointercancel", up);
  // A drag is not a pick: the click that ends it goes nowhere.
  doc.addEventListener(
    "click",
    (e) => {
      const el = target(e);
      if (el && gesture(el).moved) {
        e.stopPropagation();
        e.preventDefault();
      }
    },
    true,
  );
}
