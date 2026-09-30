// Liasse extension — a bundle of leaves turned under the thumb.
//
// Lazy-loaded runtime extension: NOT part of the core capsule. The server
// renders the bundle whole; this extension owns how it lies and moves.
//
// The DOM contract:
// - `[data-liasse]` is the bundle (one per page), marked `data-own="style
//   class"` like each of its leaves, so re-renders leave their placement to
//   this extension. The server tells:
//   - `data-bundle`: a key that changes when another bundle replaces this one
//     (the new one then comes in from the right);
//   - `data-at`: the leaf the server holds open;
//   - `data-ends`: `seal` (sealed from a leaf), `turn` (turned past its last
//     leaf) or `auto` (taken away by the table);
//   - `data-seal-at`: the leaf the seal shows on.
// - Its `.leaf` children, in order, are the leaves. A `.leaf.piece` child is
//   a paper laid over the bundle (a dossier, letters patent); `.letter`
//   makes it as tall as its text. Each leaf's text is in its `.scroll`,
//   which ends with an optional `.folio`.
// - A turn is told to the server by clicking the bundle's hidden
//   `[data-page="i"]` button; `data-page` equal to the count of leaves means
//   "past the last". A piece is laid back by clicking its `[data-piece-close]`.
// - `[data-seal]` (anywhere) is the one seal: it shows while the open leaf is
//   `data-seal-at` and no piece is laid; a touch on it ties the bundle (a
//   copy of it is folded, corded and sent left while the server answers).
//   `.cord` and `.knot` inside the bundle are its cord and wax.
// - `[data-motion]` radios (value `amples` or `sobres`) set the motions,
//   remembered on the device; `body.sober-leaves` / `body.sober-bundles`
//   carry the choice to the stylesheet.
//
// A horizontal drag anywhere on the page turns the open leaf (to the left:
// the next; to the right: the one before, drawn back by its foot); a
// vertical one scrolls the page. A drag begun on a field or a slider is the
// field's. Arrow keys turn, Escape lays a piece back.

/** Version stamp logged by the loader — bump with every change. */
export const v = 1;

type El = HTMLElement;

const installed = new WeakSet<Document>();

/** One turning gesture in progress. */
interface Drag {
  x: number;
  y: number;
  t: number;
  id: number;
  axis: "" | "x" | "y";
  dx: number;
}

export function i(doc: Document): void {
  if (installed.has(doc)) return;
  installed.add(doc);
  const win = doc.defaultView || (globalThis as unknown as Window);
  const reduce = !!(win.matchMedia && win.matchMedia("(prefers-reduced-motion: reduce)").matches);
  const STEP = 14;

  let pile: El | null = null;
  let bundle = "";
  let serverAt = -1;
  let at = 0;
  let leaves: El[] = [];
  let piece: El | null = null;
  let busy = false;
  let ghost: El | null = null;
  let drag: Drag | null = null;
  let dragged = false;

  // --- motions --------------------------------------------------------------
  let mode = "amples";
  try {
    mode = win.localStorage.getItem("liasse-motion") === "sobres" ? "sobres" : "amples";
  } catch (e) {
    /* storage refused: the default stands */
  }
  let soberLeaves = false;
  const setMotion = () => {
    soberLeaves = mode === "sobres";
    doc.body.classList.toggle("sober-leaves", soberLeaves);
    doc.body.classList.add("sober-bundles");
    doc.querySelectorAll<HTMLInputElement>("[data-motion]").forEach((r) => {
      r.checked = r.value === mode;
    });
  };
  const T = (ample: number, sober: number, bundles?: boolean) =>
    reduce ? 0 : bundles || soberLeaves ? sober : ample;

  const width = () => (pile ? pile.getBoundingClientRect().width : win.innerWidth);
  const ends = () => (pile && pile.dataset.ends) || "seal";
  const sealAt = () => {
    const s = pile && pile.dataset.sealAt;
    return s === undefined || s === null || s === "" ? -1 : +s;
  };

  // --- where each leaf lies -------------------------------------------------
  function rest(i: number, dx: number): string {
    const w = width();
    const N = leaves.length;
    if (i < at) {
      // Turned leaves wait to the left, their lower corner tilted into view
      // at the foot of the screen; drawn back, the leaf straightens.
      const off = -w * 1.08;
      const back = i === at - 1 && dx > 0 ? Math.min(dx, -off) : 0;
      return `translate3d(${off + back}px,0,0) rotate(${-7 * (1 + back / off)}deg)`;
    }
    if (i === at) {
      let d = dx < 0 ? dx : 0;
      // The last leaf turns away only where turning ends the bundle.
      if (at === N - 1 && dx < 0 && ends() !== "turn") d = dx * 0.25;
      if (at === 0 && dx > 0) d = dx * 0.25;
      return `translate3d(${d}px,0,0) rotate(${(d / w) * (soberLeaves ? 1.5 : 6)}deg)`;
    }
    const depth = i - at;
    const lift = dx < 0 && depth === 1 ? Math.min(1, -dx / w) : 0;
    const k = depth - lift;
    return `translate3d(${k * 5}px,${k * STEP}px,0)`;
  }

  /** Each turned leaf's foot sits on the foot of the screen, never below the
   * bundle's own foot (a turned leaf must not lengthen the page). */
  function footTurned(): void {
    if (!pile) return;
    const open = leaves[at];
    const foot = Math.min(win.innerHeight - pile.getBoundingClientRect().top, open ? open.offsetHeight : 0);
    leaves.forEach((leaf, i) => {
      leaf.style.top = i < at ? foot - leaf.offsetHeight + "px" : "";
    });
  }

  function paint(dx: number, animate: boolean): void {
    if (!pile) return;
    const N = leaves.length;
    leaves.forEach((leaf, i) => {
      const c = leaf.classList;
      c.toggle("animating", animate);
      c.toggle("open", i === at);
      c.toggle("under", i > at + 1);
      c.toggle("deep", i > at + 1);
      c.toggle("next", i === at + 1);
      c.toggle("turned", i < at);
      // The open leaf lies over the turned ones, so a leaf turned in haste
      // never slides under the one still flying off.
      leaf.style.zIndex = String(i < at ? 20 + i : i === at ? 45 : 40 - (i - at));
      leaf.style.transform = rest(i, dx);
      leaf.setAttribute("aria-hidden", i === at && !piece ? "false" : "true");
      (leaf as El & { inert: boolean }).inert = i !== at || !!piece;
    });
    // A leaf drawn back comes over everything.
    if (dx > 0 && at > 0) leaves[at - 1].style.zIndex = "60";
    footTurned();
    pile.classList.add("pile-edges");
    pile.style.setProperty("--edges", (N - 1 - at) * STEP + 14 + "px");
    showSeal();
  }

  // --- the seal -------------------------------------------------------------
  function showSeal(): void {
    const here = !!pile && !busy && !piece && !ghost && ends() === "seal" && at === sealAt();
    doc.querySelectorAll<El>("[data-seal]").forEach((s) => {
      s.hidden = false;
      s.classList.toggle("hidden-away", !here);
    });
  }

  /** Tell the server which leaf lies open (or that the last was turned past). */
  function tell(i: number): void {
    const b = pile && pile.querySelector<El>(`[data-page="${i}"]`);
    if (b) b.click();
    serverAt = i;
  }

  // --- turning --------------------------------------------------------------
  function go(i: number): void {
    if (busy || !pile) return;
    const N = leaves.length;
    if (i === N && ends() === "turn") {
      // Turning past the last leaf: it flies off, and the bundle goes.
      const last = leaves[at];
      last.classList.add("animating");
      last.style.transform = `translate3d(${-width() * (soberLeaves ? 0.5 : 1.1)}px,0,0) rotate(${soberLeaves ? -1.5 : -6}deg)`;
      if (soberLeaves) last.style.opacity = "0";
      win.setTimeout(() => {
        depart();
        tell(N);
      }, T(300, 220));
      return;
    }
    if (i < 0 || i >= N || i === at) {
      paint(0, true);
      return;
    }
    if (i < at && soberLeaves && !reduce) {
      // Sober: the screen goes to the top at once, and the leaf fades in with
      // a short slide from the left; the one that was open settles under it.
      const back = leaves[i];
      const settling = leaves[at];
      at = i;
      paint(0, false);
      win.scrollTo(0, 0);
      settling.style.opacity = "0";
      win.setTimeout(() => (settling.style.opacity = ""), 240);
      back.style.opacity = "0";
      back.style.transform = "translate3d(-24px,0,0)";
      void back.offsetHeight;
      back.classList.add("animating");
      back.style.opacity = "";
      back.style.transform = rest(at, 0);
    } else if (i < at) {
      // Drawn back: the screen goes to the top at once, and the leaf, held
      // where it was seen, glides to its place in one move, across and up;
      // the one that was open settles into the pile under it.
      const leaf = leaves[i];
      const left = leaves[at];
      const from = pile.getBoundingClientRect().top + (parseFloat(leaf.style.top) || 0);
      const was = leaf.style.transform;
      const leftRect = left.getBoundingClientRect();
      at = i;
      paint(0, false);
      win.scrollTo(0, 0);
      const dy = from - pile.getBoundingClientRect().top;
      leaf.style.transform = `translate3d(0,${dy}px,0) ${was}`;
      const settled = left.getBoundingClientRect();
      left.style.transform = `translate3d(${leftRect.left - settled.left}px,${leftRect.top - settled.top}px,0) ${rest(at + 1, 0)}`;
      void leaf.offsetHeight;
      for (const l of [leaf, left]) {
        l.classList.add("animating");
        l.style.transitionDuration = reduce ? "" : ".52s";
      }
      leaf.style.transform = rest(at, 0);
      left.style.transform = rest(at + 1, 0);
      win.setTimeout(() => {
        leaf.style.transitionDuration = "";
        left.style.transitionDuration = "";
      }, 560);
    } else {
      // The leaf turned away stays on top while it flies off, then lies under
      // the pile; a drag begun meanwhile repaints and puts the dragged leaf over it.
      const gone = leaves[at];
      at = i;
      paint(0, true);
      gone.style.zIndex = "60";
      if (soberLeaves) gone.style.opacity = "0";
      win.setTimeout(() => {
        const k = leaves.indexOf(gone);
        if (k >= 0 && k < at && !drag) gone.style.zIndex = String(20 + k);
        gone.style.opacity = "";
      }, T(420, 240));
      win.scrollTo({ top: 0, behavior: reduce || soberLeaves ? "auto" : "smooth" });
    }
    tell(at);
  }

  // --- a bundle comes, a bundle goes ----------------------------------------
  function arrive(): void {
    if (!pile) return;
    busy = true;
    pile.style.visibility = "";
    pile.style.height = "";
    pile.className = "pile pile-edges above";
    paint(0, false);
    win.scrollTo(0, 0);
    // The table stays bare a moment, then the bundle comes in from the right.
    win.setTimeout(() => {
      win.requestAnimationFrame(() => {
        if (!pile) return;
        pile.className = "pile pile-edges";
        win.setTimeout(() => {
          busy = false;
          showSeal();
        }, T(900, 300, true));
      });
    }, T(450, 150, true));
  }

  /** The bundle done: a copy of it gathers its leaves, is tied (a single
   * leaf is only sealed at its corner) and goes out to the left, while the
   * real one, hidden, receives the next from the server. */
  function depart(): void {
    if (!pile || ghost) return;
    busy = true;
    showSeal();
    win.scrollTo(0, 0);
    const r = pile.getBoundingClientRect();
    const g = pile.cloneNode(true) as El;
    g.removeAttribute("data-liasse");
    g.removeAttribute("id");
    g.querySelectorAll("[id],[data-page],[data-seal],[data-piece-close]").forEach((e) => {
      if (e.hasAttribute("data-page") || e.hasAttribute("data-seal") || e.hasAttribute("data-piece-close")) e.remove();
      else e.removeAttribute("id");
    });
    g.querySelectorAll(".leaf.piece").forEach((e) => e.remove());
    g.setAttribute("aria-hidden", "true");
    g.style.position = "fixed";
    g.style.left = r.left + "px";
    g.style.top = r.top + "px";
    g.style.width = r.width + "px";
    g.style.margin = "0";
    g.style.zIndex = "70";
    g.style.height = Math.min(pile.offsetHeight, win.innerHeight - 50) + "px";
    doc.body.appendChild(g);
    ghost = g;
    pile.style.visibility = "hidden";
    const ls = Array.from(g.querySelectorAll<El>(":scope > .leaf"));
    const single = ls.length === 1;
    g.classList.add("folding");
    ls.forEach((leaf, i) => {
      leaf.style.top = "";
      leaf.classList.remove("open", "next", "turned", "animating");
      leaf.classList.toggle("deep", i > 0);
      leaf.classList.toggle("under", i > 0);
      leaf.style.transform = `translate3d(${i * 2}px,${i * 2}px,0)`;
      leaf.style.zIndex = String(40 - i);
      leaf.style.opacity = "";
    });
    const leaving = pile.dataset.bundle || "";
    const folded = single ? T(120, 60, true) : T(500, 250, true);
    const sent = single ? T(1300, 850, true) : T(2150, 1150, true);
    const gone = single ? T(2250, 1250, true) : T(3100, 1550, true);
    if (single) {
      g.classList.add("single");
      g.classList.toggle("unsealed", ends() !== "seal");
    }
    win.setTimeout(() => g.classList.add("folded"), folded);
    win.setTimeout(() => g.classList.add("sent"), reduce ? 50 : sent);
    win.setTimeout(() => {
      g.remove();
      ghost = null;
      // The next bundle comes as soon as the server has sent it; if the
      // server kept this one (a refusal), it comes back.
      if (pile && (pile.dataset.bundle || "") === leaving) {
        win.setTimeout(() => {
          if (pile && (pile.dataset.bundle || "") === leaving) arrive();
        }, 1200);
      } else arrive();
    }, reduce ? 150 : gone);
  }

  // --- pieces ---------------------------------------------------------------
  function layPiece(p: El): void {
    piece = p;
    if (!pile) return;
    pile.style.setProperty("--piece-min", pile.offsetHeight + "px");
    p.style.transform = soberLeaves ? "translate3d(28px,0,0)" : `translate3d(${width() * 1.08}px,0,0)`;
    if (soberLeaves) p.style.opacity = "0";
    win.scrollTo(0, 0);
    void p.offsetHeight;
    p.classList.add("moving");
    p.style.transform = "";
    p.style.opacity = "";
    paint(0, false);
  }

  function liftPiece(): void {
    const p = piece;
    if (!p) return;
    piece = null;
    p.classList.add("moving");
    p.style.transform = soberLeaves ? "translate3d(28px,0,0)" : `translate3d(${width() * 1.08}px,0,0)`;
    if (soberLeaves) p.style.opacity = "0";
    win.setTimeout(() => {
      const close = p.querySelector<El>("[data-piece-close]");
      if (close) close.click();
    }, T(300, 300));
    paint(0, false);
  }

  // --- following the server -------------------------------------------------
  function sync(): void {
    const p = doc.querySelector<El>("[data-liasse]");
    if (!p) {
      pile = null;
      showSeal();
      return;
    }
    const key = p.dataset.bundle || "";
    const fresh = p !== pile || key !== bundle;
    pile = p;
    const now = Array.from(p.querySelectorAll<El>(":scope > .leaf:not(.piece)"));
    // Only a new set of leaves is laid again: a re-render of the same ones
    // must not cut a turn in flight.
    const changed = now.length !== leaves.length || now.some((l, k) => l !== leaves[k]);
    leaves = now;
    const sAt = +(p.dataset.at || 0);
    if (fresh) {
      bundle = key;
      at = Math.min(sAt, Math.max(0, leaves.length - 1));
      serverAt = sAt;
      piece = null;
      if (!ghost) arrive();
      else pile.style.visibility = "hidden";
    } else if (sAt !== serverAt && !drag) {
      // The server moved the bundle (a reconnection, another device).
      serverAt = sAt;
      if (sAt !== at && sAt < leaves.length) {
        at = sAt;
        paint(0, true);
      }
    }
    const pc = p.querySelector<El>(":scope > .leaf.piece");
    if (pc && pc !== piece) layPiece(pc);
    else if (!pc && piece) piece = null;
    if (changed && !fresh && !drag && !ghost) paint(0, false);
    setMotion();
  }

  // --- the thumb ------------------------------------------------------------
  doc.addEventListener("pointerdown", (e) => {
    if (busy || !pile || (e.pointerType === "mouse" && e.button !== 0)) return;
    const t = e.target as Element | null;
    if (t && t.closest && t.closest("input, textarea, select, [data-no-turn]")) return;
    drag = { x: e.clientX, y: e.clientY, t: e.timeStamp, id: e.pointerId, axis: "", dx: 0 };
  });
  doc.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.id || !pile) return;
    const dx = e.clientX - drag.x,
      dy = e.clientY - drag.y;
    if (!drag.axis) {
      if (Math.abs(dx) < 8 && Math.abs(dy) < 8) return;
      drag.axis = Math.abs(dx) > Math.abs(dy) ? "x" : "y";
      if (drag.axis === "x") {
        try {
          doc.documentElement.setPointerCapture(e.pointerId);
        } catch (err) {
          /* capture refused: the move still follows */
        }
      }
    }
    if (drag.axis !== "x") return;
    drag.dx = dx;
    if (piece) {
      piece.classList.remove("moving");
      piece.style.transform = `translate3d(${Math.max(0, dx)}px,0,0)`;
      return;
    }
    pile.classList.add("dragging");
    paint(dx, false);
  });
  const release = (e: PointerEvent) => {
    if (!drag || e.pointerId !== drag.id) return;
    const s = drag;
    drag = null;
    if (s.axis !== "x" || !pile) return;
    // The leaves keep their text until the move they started is over.
    win.setTimeout(() => {
      if (!drag && pile) pile.classList.remove("dragging");
    }, 620);
    const w = width();
    const v = s.dx / Math.max(1, e.timeStamp - s.t);
    if (piece) {
      if (s.dx > w * 0.25 || v > 0.5) liftPiece();
      else {
        piece.classList.add("moving");
        piece.style.transform = "";
      }
    } else if (s.dx < -w * 0.25 || v < -0.5) go(at + 1);
    else if (s.dx > w * 0.25 || v > 0.5) go(at - 1);
    else paint(0, true);
    // A drag is no tap: the button under the thumb is not pressed.
    dragged = true;
    win.setTimeout(() => (dragged = false), 50);
  };
  doc.addEventListener("pointerup", release);
  doc.addEventListener("pointercancel", release);

  doc.addEventListener(
    "click",
    (e) => {
      if (dragged) {
        e.stopPropagation();
        e.preventDefault();
        return;
      }
      const t = e.target as Element | null;
      const seal = t && t.closest && (t.closest("[data-seal]") as El | null);
      if (seal && pile && !busy && !seal.classList.contains("hidden-away")) {
        // A seal that submits a form ties nothing the form would refuse.
        const f = seal.getAttribute("form");
        const form = f ? (doc.getElementById(f) as HTMLFormElement | null) : null;
        if (form && !form.checkValidity()) return;
        // The floating seal fades where it is; the bundle is tied with its own.
        seal.blur();
        seal.classList.add("fading");
        win.setTimeout(() => {
          seal.classList.remove("fading");
          seal.classList.add("hidden-away");
        }, T(400, 400, true));
        depart();
      }
    },
    true,
  );

  doc.addEventListener("keydown", (e) => {
    if (!pile) return;
    const t = e.target as Element | null;
    if (t && t.closest && t.closest("input, textarea, select")) return;
    if (piece) {
      if (e.key === "Escape" || e.key === "ArrowLeft") liftPiece();
      return;
    }
    if (e.key === "ArrowRight") go(at + 1);
    if (e.key === "ArrowLeft") go(at - 1);
  });

  doc.addEventListener("change", (e) => {
    const r = e.target as HTMLInputElement | null;
    if (!r || !r.hasAttribute || !r.hasAttribute("data-motion")) return;
    mode = r.value === "sobres" ? "sobres" : "amples";
    try {
      win.localStorage.setItem("liasse-motion", mode);
    } catch (err) {
      /* storage refused: the choice holds for this page */
    }
    setMotion();
  });

  let queued = false;
  win.addEventListener(
    "scroll",
    () => {
      if (queued || at === 0) return;
      queued = true;
      win.requestAnimationFrame(() => {
        queued = false;
        footTurned();
      });
    },
    { passive: true },
  );
  win.addEventListener("resize", () => {
    if (!busy && !drag) paint(0, false);
  });

  if (typeof MutationObserver !== "undefined")
    new MutationObserver(() => sync()).observe(doc.body || doc, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ["data-at", "data-bundle", "data-ends", "data-seal-at"],
    });
  setMotion();
  sync();
}

if (typeof document !== "undefined") i(document);
