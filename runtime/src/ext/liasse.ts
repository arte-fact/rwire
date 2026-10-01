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
//   - `data-ends`: `seal` (signed from a leaf), `turn` (turned past its last
//     leaf) or `auto` (taken away by the table).
// - Its `.leaf` children, in order, are the leaves. A `.leaf.piece` child is
//   a paper laid over the bundle (a dossier, letters patent); `.letter`
//   makes it as tall as its text, and laying it leaves the page's scroll
//   alone (the stylesheet sets it at the foot of the screen). Each leaf's text is in its `.scroll`,
//   which ends with an optional `.folio`.
// - A turn is told to the server by clicking the bundle's hidden
//   `[data-page="i"]` button; `data-page` equal to the count of leaves means
//   "past the last". A piece is laid back by clicking its `[data-piece-close]`.
// - `[data-sign]` is the signature line at the foot of the leaf a bundle is
//   signed from: touched, its `.hand` (the seigneur's name, or the value of
//   the input `data-sign-from` names) is written by hand (`.signing`,
//   `--dur` set to the pen's time), then the button's own action goes
//   (a click, or the form it submits) and the bundle slides off to the left
//   while the server answers; the next comes in from the right. A bundle
//   the server keeps (a refusal) slides back.
// - A submit button `[data-stamp="F"]` is a cachet: pressed, it prints
//   `<span class="imprint">F</span>` on its leaf (`.stamped`), then submits
//   its form a moment later; a piece so sealed goes off to the right while the
//   server answers. The imprint fades if the server keeps the paper.
// - `[data-motion]` radios (value `amples` or `sobres`) set the motions,
//   remembered on the device; `body.sober-leaves` / `body.sober-bundles`
//   carry the choice to the stylesheet.
//
// A horizontal drag anywhere on the page turns the open leaf (to the left:
// the next slides in under it; to the right: the one before slides back over
// it), flat, at the thumb's own speed; a vertical one scrolls the page. The
// leaf coming in is laid with its head at the top of the screen wherever the
// page is scrolled, and only once it has settled does it become the open
// leaf, the page put back to its top in the same instant: nothing is seen
// to scroll. A drag begun on a field or a slider is the field's. Arrow keys
// turn, Escape lays a piece back.

/** Version stamp logged by the loader — bump with every change. */
export const v = 6;

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
  // The edges of the leaves under the open one: a few pixels each, two at most.
  const STEP = 6,
    EDGES = 2;

  let pile: El | null = null;
  let bundle = "";
  let serverAt = -1;
  let at = 0;
  let leaves: El[] = [];
  let piece: El | null = null;
  let busy = false;
  // When the bundle slid off (0: it did not), and which bundle it was.
  let sentAt = 0;
  let leaving = "";
  let drag: Drag | null = null;
  let dragged = false;
  // The page's scroll when a turn began: the leaf coming in is laid that far
  // down, so its head meets the top of the screen.
  let align = 0;
  // A slide under way: the swap it ends with, and its timer.
  let settle: (() => void) | null = null;
  let settling: ReturnType<typeof setTimeout> | undefined;
  let resting: ReturnType<typeof setTimeout> | undefined;

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

  // --- where each leaf lies -------------------------------------------------
  function rest(i: number, dx: number): string {
    const w = width();
    const N = leaves.length;
    // Flat: no leaf tilts. A leaf on its way in (drawn back, or lifted as
    // the open one goes) is laid with its head at the top of the screen.
    if (i < at) {
      // Turned leaves wait wholly off-screen to the left.
      const off = -w * 1.04;
      const back = i === at - 1 && dx > 0 ? Math.min(dx, -off) : 0;
      return `translate3d(${off + back}px,${back > 0 ? align : 0}px,0)`;
    }
    if (i === at) {
      let d = dx < 0 ? dx : 0;
      // The last leaf slides away only where turning ends the bundle.
      if (at === N - 1 && dx < 0 && ends() !== "turn") d = dx * 0.25;
      if (at === 0 && dx > 0) d = dx * 0.25;
      return `translate3d(${d}px,0,0)`;
    }
    const depth = Math.min(i - at, EDGES);
    const lift = dx < 0 && depth === 1 ? Math.min(1, -dx / w) : 0;
    const k = depth - lift;
    return `translate3d(${k * 3}px,${k * STEP + (lift > 0 ? align : 0)}px,0)`;
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
      // Buried leaves are not drawn at all.
      leaf.style.visibility = i > at + EDGES || i < at - 1 ? "hidden" : "";
      leaf.setAttribute("aria-hidden", i === at && !piece ? "false" : "true");
      (leaf as El & { inert: boolean }).inert = i !== at || !!piece;
    });
    // A leaf drawn back comes over everything.
    if (dx > 0 && at > 0) leaves[at - 1].style.zIndex = "60";
    pile.classList.add("pile-edges");
    pile.style.setProperty("--edges", Math.min(N - 1 - at, EDGES) * STEP + 8 + "px");
  }

  /** The neighbours' text is hidden again only once no thumb and no slide
   * is at work: a timer from an earlier turn never hides a leaf under way. */
  function restText(): void {
    win.clearTimeout(resting);
    resting = win.setTimeout(() => {
      if (!drag && !settle && pile) pile.classList.remove("dragging");
    }, 400);
  }

  /** Tell the server which leaf lies open (or that the last was turned past). */
  function tell(i: number): void {
    const b = pile && pile.querySelector<El>(`[data-page="${i}"]`);
    if (b) b.click();
    serverAt = i;
  }

  // --- turning --------------------------------------------------------------
  /** Turn to leaf `i`; `dur` is the time the slide takes, the thumb's speed
   * leaving it, a beat at least. */
  function go(i: number, dur?: number): void {
    if (busy || !pile) return;
    const N = leaves.length;
    dur = Math.round(dur || T(340, 220));
    if (i === N && ends() === "turn") {
      // Turning past the last leaf: it flies off, and the bundle goes.
      const last = leaves[at];
      last.classList.add("animating");
      last.style.transform = `translate3d(${-width() * (soberLeaves ? 0.5 : 1.04)}px,0,0)`;
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
    // Flat: the leaves slide across, the one coming in laid with its head at
    // the top of the screen. Only when it has settled does it become the
    // open leaf, and the page is put back to its top in the same instant:
    // the screen shows the same thing before and after, so nothing is seen
    // to scroll.
    if (!drag) align = Math.max(0, Math.round(win.scrollY));
    pile.classList.add("dragging");
    const coming = leaves[i];
    const moving = [coming];
    coming.classList.add("animating");
    if (i < at) {
      // Drawn back over the open leaf.
      coming.style.zIndex = "60";
      coming.style.transform = `translate3d(0,${align}px,0)`;
    } else {
      // The open leaf slides away; the next, lifted, takes its place.
      const gone = leaves[at];
      gone.classList.add("animating");
      moving.push(gone);
      gone.style.zIndex = "60";
      gone.style.transform = `translate3d(${-width() * 1.04}px,0,0)`;
      coming.style.transform = `translate3d(0,${align}px,0)`;
      leaves.forEach((l, k) => {
        if (k > i) {
          l.classList.add("animating");
          moving.push(l);
          const m = Math.min(k - i, EDGES);
          l.style.transform = `translate3d(${m * 3}px,${m * STEP}px,0)`;
        }
      });
    }
    for (const l of moving) l.style.transitionDuration = dur + "ms";
    // The swap and the scroll in one frame, unseen; a thumb landing before
    // the slide is over settles it at once and goes on from there.
    settle = () => {
      win.clearTimeout(settling);
      settle = null;
      at = i;
      align = 0;
      for (const l of moving) l.style.transitionDuration = "";
      paint(0, false);
      win.scrollTo(0, 0);
      // At the swap the leaf passed behind hides its text at once: the
      // edges that stick out show bare paper.
      if (!drag && pile) pile.classList.remove("dragging");
    };
    settling = win.setTimeout(settle, dur);
    // The server is told the leaf turned to at once, while it slides in.
    tell(i);
  }

  // --- a bundle comes, a bundle goes ----------------------------------------
  const OUT = () => T(420, 300, true);
  /** The new bundle: once the one before has slid off, the table stays bare
   * a moment, then it slides in from the right. */
  function arrive(): void {
    if (!pile) return;
    busy = true;
    const wait = sentAt ? Math.max(0, sentAt + OUT() - Date.now()) : 0;
    sentAt = 0;
    win.setTimeout(() => {
      if (!pile) return;
      pile.className = "pile pile-edges above";
      paint(0, false);
      win.scrollTo(0, 0);
      win.setTimeout(() => {
        win.requestAnimationFrame(() => {
          if (!pile) return;
          pile.className = "pile pile-edges";
          win.setTimeout(() => {
            busy = false;
          }, T(550, 300, true));
        });
      }, T(300, 150, true));
    }, wait);
  }

  /** The bundle done: it slides off to the left while the server answers;
   * kept by the server (a refusal), it slides back. */
  function depart(): void {
    if (!pile || sentAt) return;
    busy = true;
    sentAt = Date.now();
    leaving = pile.dataset.bundle || "";
    pile.classList.add("sent");
    win.setTimeout(() => {
      if (pile && sentAt && (pile.dataset.bundle || "") === leaving) arrive();
    }, OUT() + 1200);
  }

  // --- pieces ---------------------------------------------------------------
  function layPiece(p: El): void {
    piece = p;
    if (!pile) return;
    pile.style.setProperty("--piece-min", pile.offsetHeight + "px");
    p.style.transform = soberLeaves ? "translate3d(28px,0,0)" : `translate3d(${width() * 1.08}px,0,0)`;
    if (soberLeaves) p.style.opacity = "0";
    // A letter lies at the foot of the screen, out of the page's scroll: the
    // page stays where the reader was.
    if (!p.classList.contains("letter")) win.scrollTo(0, 0);
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

  // The seigneur's name written on the line, then the line's own action.
  function signWith(sign: HTMLButtonElement): void {
    const hand = sign.querySelector<El>(".hand");
    const from = sign.getAttribute("data-sign-from");
    const input = from ? (doc.getElementById(from) as HTMLInputElement | null) : null;
    const name = (input && input.value.trim()) || (hand && hand.textContent) || "";
    if (hand) hand.textContent = name;
    // The pen takes about a tenth of a second a letter.
    const dur = reduce ? 10 : Math.min(1800, Math.max(700, 95 * name.length));
    sign.style.setProperty("--dur", dur + "ms");
    sign.classList.add("signing");
    sign.blur();
    busy = true;
    win.setTimeout(() => {
      busy = false;
      sign.setAttribute("data-signed", "");
      sign.click();
      depart();
    }, dur + (reduce ? 50 : 700));
  }

  // A cachet pressed on its leaf, then the form it seals submitted.
  function stampWith(stamp: HTMLButtonElement): void {
    const leaf = (stamp.closest(".leaf") as El | null) || (stamp.parentElement as El);
    const mark = doc.createElement("span");
    mark.className = "imprint";
    mark.textContent = stamp.getAttribute("data-stamp") || "";
    leaf.appendChild(mark);
    leaf.classList.add("stamped");
    stamp.setAttribute("data-stamped", "");
    win.setTimeout(() => {
      stamp.click();
      stamp.removeAttribute("data-stamped");
      if (piece === leaf) {
        piece = null;
        leaf.classList.add("moving");
        leaf.style.transform = soberLeaves ? "translate3d(28px,0,0)" : `translate3d(${width() * 1.08}px,0,0)`;
        leaf.style.opacity = "0";
        paint(0, false);
      }
      // Kept by the server (a refusal in the margin): the wax lifts.
      win.setTimeout(() => {
        if (!leaf.isConnected) return;
        mark.remove();
        leaf.classList.remove("stamped");
        leaf.style.opacity = "";
        if (!piece && leaf.classList.contains("piece")) layPiece(leaf);
      }, 1600);
    }, T(560, 260, true));
  }

  // --- following the server -------------------------------------------------
  function sync(): void {
    const p = doc.querySelector<El>("[data-liasse]");
    if (!p) {
      pile = null;
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
      arrive();
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
    if (changed && !fresh && !drag && !sentAt) paint(0, false);
    setMotion();
  }

  // --- the thumb ------------------------------------------------------------
  doc.addEventListener("pointerdown", (e) => {
    if (busy || !pile || (e.pointerType === "mouse" && e.button !== 0)) return;
    const t = e.target as Element | null;
    if (t && t.closest && t.closest("input, textarea, select, [data-no-turn]")) return;
    if (settle) settle();
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
        align = Math.max(0, Math.round(win.scrollY));
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
    const w = width();
    const v = s.dx / Math.max(1, e.timeStamp - s.t);
    // The way left to go, at the thumb's own speed: a flick ends fast.
    const dur = Math.max(120, Math.min(T(340, 220), (w - Math.min(w, Math.abs(s.dx))) / Math.max(0.8, Math.abs(v))));
    if (piece) {
      if (s.dx > w * 0.25 || v > 0.5) liftPiece();
      else {
        piece.classList.add("moving");
        piece.style.transform = "";
      }
    } else if (s.dx < -w * 0.25 || v < -0.5) go(at + 1, dur);
    else if (s.dx > w * 0.25 || v > 0.5) go(at - 1, dur);
    else {
      align = 0;
      paint(0, true);
      restText();
    }
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
      const stamp = t && t.closest && (t.closest("[data-stamp]") as HTMLButtonElement | null);
      if (stamp && !stamp.hasAttribute("data-stamped")) {
        if (stamp.form && !stamp.form.checkValidity()) return;
        e.preventDefault();
        e.stopPropagation();
        stampWith(stamp);
        return;
      }
      const sign = t && t.closest && (t.closest("[data-sign]") as HTMLButtonElement | null);
      if (sign && pile && !sign.hasAttribute("data-signed")) {
        if (busy || sign.classList.contains("signing")) {
          e.preventDefault();
          e.stopPropagation();
          return;
        }
        // A line that submits a form signs nothing the form would refuse.
        if (sign.form && !sign.form.checkValidity()) return;
        e.preventDefault();
        e.stopPropagation();
        signWith(sign);
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

  win.addEventListener("resize", () => {
    if (!busy && !drag) paint(0, false);
  });

  if (typeof MutationObserver !== "undefined")
    new MutationObserver(() => sync()).observe(doc.body || doc, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ["data-at", "data-bundle", "data-ends"],
    });
  setMotion();
  sync();
}

if (typeof document !== "undefined") i(document);
