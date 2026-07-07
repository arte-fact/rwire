// Vim extension — modal text editing for rwire editors (M2 engine).
//
// Lazy-loaded runtime extension (docs/vim-mode-design.md rev.4): NOT part of
// the core capsule. Speaks to the rest of the system only through the DOM
// contract — reads/writes the `data-vim` mode attribute, updates the
// `[data-vim-chip]` label, moves the native caret/selection, and dispatches
// synthetic bubbling `input` events after every mutation so the delegated
// dispatcher, overlay echo, dirty diff, and autosave all flow unchanged.
// Undo/redo delegate to the server history by clicking the [data-kbd]
// elements. Pending counts/operators and the unnamed register are
// input-method state and live here by design. Motions for TEXT EDITING only:
// no ex commands, ever.

type Mode = "normal" | "insert" | "v" | "V";
interface Pend {
  count: string;
  op: "" | "d" | "c" | "y" | "g";
  anchor: number; // visual-mode anchor
  head: number; // visual-mode cursor (selection ends can't encode it in V)
}

const P = new WeakMap<Element, Pend>();
let reg = "";
let regLine = false;

const pend = (el: Element): Pend => {
  let p = P.get(el);
  if (!p) {
    p = { count: "", op: "", anchor: 0, head: 0 };
    P.set(el, p);
  }
  return p;
};

// ---------------------------------------------------------------- text nav
const lineStart = (t: string, p: number): number => t.lastIndexOf("\n", p - 1) + 1;
const lineEnd = (t: string, p: number): number => {
  const n = t.indexOf("\n", p);
  return n === -1 ? t.length : n;
};
const firstNonBlank = (t: string, p: number): number => {
  const s = lineStart(t, p);
  const e = lineEnd(t, s);
  let i = s;
  while (i < e && (t[i] === " " || t[i] === "\t")) i++;
  return i;
};
const isW = (c: string): boolean => /\w/.test(c);
const cls = (c: string): number => (isW(c) ? 1 : c === " " || c === "\t" || c === "\n" ? 0 : 2);

function wordFwd(t: string, p: number): number {
  if (p >= t.length) return p;
  const k = cls(t[p]);
  if (k !== 0) while (p < t.length && cls(t[p]) === k) p++;
  while (p < t.length && cls(t[p]) === 0) p++;
  return p;
}
function wordBack(t: string, p: number): number {
  if (p <= 0) return 0;
  p--;
  while (p > 0 && cls(t[p]) === 0) p--;
  const k = cls(t[p]);
  while (p > 0 && cls(t[p - 1]) === k) p--;
  return p;
}
function wordEnd(t: string, p: number): number {
  p++;
  while (p < t.length && cls(t[p]) === 0) p++;
  if (p >= t.length) return t.length ? t.length - 1 : 0;
  const k = cls(t[p]);
  while (p + 1 < t.length && cls(t[p + 1]) === k) p++;
  return p;
}
function lineMove(t: string, p: number, dir: 1 | -1): number {
  const s = lineStart(t, p);
  const col = p - s;
  if (dir === 1) {
    const e = lineEnd(t, p);
    if (e === t.length) return p;
    const ns = e + 1;
    return Math.min(ns + col, lineEnd(t, ns));
  }
  if (s === 0) return p;
  const ps = lineStart(t, s - 1);
  return Math.min(ps + col, lineEnd(t, ps));
}

// -------------------------------------------------------------- dispatch
type TA = HTMLTextAreaElement;

function fire(el: TA): void {
  if ((el as any).dispatchEvent) el.dispatchEvent(new Event("input", { bubbles: true }));
  else (el as any).fire && (el as any).fire("input", { target: el });
}

function put(el: TA, text: string, caret: number): void {
  el.value = text;
  el.setSelectionRange(caret, caret);
  fire(el);
  if ((el as any).focus) el.focus();
}

function setMode(doc: Document, el: TA, m: Mode): void {
  el.setAttribute("data-vim", m);
  const chip = doc.querySelector("[data-vim-chip]");
  if (chip) {
    chip.textContent = m === "v" ? "VISUAL" : m === "V" ? "V-LINE" : m.toUpperCase();
    chip.setAttribute("data-vim-mode", m);
  }
}

function clickKbd(doc: Document, combo: string): void {
  const t = doc.querySelector('[data-kbd="' + combo + '"]') as HTMLElement | null;
  if (t) t.click();
}

/** Apply a motion key from `p`; returns the new position or -1 if not a motion. */
function motion(t: string, p: number, k: string, n: number, hasCount: boolean): number {
  let q = p;
  for (let i = 0; i < n; i++) {
    switch (k) {
      case "h": q = Math.max(lineStart(t, q), q - 1); break;
      case "l": q = Math.min(lineEnd(t, q), q + 1); break;
      case "j": q = lineMove(t, q, 1); break;
      case "k": q = lineMove(t, q, -1); break;
      case "w": q = wordFwd(t, q); break;
      case "b": q = wordBack(t, q); break;
      case "e": q = wordEnd(t, q); break;
      case "0": return lineStart(t, p);
      case "^": return firstNonBlank(t, p);
      case "$": q = lineEnd(t, q); if (i < n - 1 && q < t.length) q++; break;
      case "G":
        return hasCount ? lineForCount(t, n) : firstNonBlank(t, lineStart(t, t.length));
      default: return -1;
    }
  }
  return q;
}
const lineForCount = (t: string, n: number): number => {
  let p = 0;
  for (let i = 1; i < n; i++) {
    const e = lineEnd(t, p);
    if (e === t.length) break;
    p = e + 1;
  }
  return firstNonBlank(t, p);
};

/** Line-wise span [start, endExcl) covering the lines of [a,b]. */
function lineSpan(t: string, a: number, b: number): [number, number] {
  const s = lineStart(t, Math.min(a, b));
  let e = lineEnd(t, Math.max(a, b));
  if (e < t.length) e++; // include the newline
  return [s, e];
}

function yank(text: string, linewise: boolean): void {
  reg = text;
  regLine = linewise;
}

// ------------------------------------------------------------------ keys
function normalKey(doc: Document, el: TA, k: string): void {
  const t = el.value;
  const p = el.selectionStart;
  const st = pend(el);
  const n = st.count ? parseInt(st.count, 10) : 1;

  // counts (0 is a motion when no count pending)
  if (/^[1-9]$/.test(k) || (k === "0" && st.count)) {
    st.count += k;
    return;
  }

  // gg / operator-doubling
  if (st.op === "g") {
    st.op = "";
    if (k === "g") {
      const q = st.count ? lineForCount(t, n) : firstNonBlank(t, 0);
      st.count = "";
      el.setSelectionRange(q, q);
    }
    return;
  }
  if (st.op && k === st.op) {
    // dd / yy / cc — n lines
    let [s, e] = lineSpan(t, p, p);
    for (let i = 1; i < n; i++) e = lineSpan(t, e, e)[1];
    yank(t.slice(s, e), true);
    st.count = "";
    const op = st.op;
    st.op = "";
    if (op === "y") {
      el.setSelectionRange(s, s);
      return;
    }
    if (op === "c") {
      put(el, t.slice(0, s) + t.slice(e), s);
      setMode(doc, el, "insert");
      return;
    }
    if (e === t.length && s > 0) s--; // deleting through EOF eats the leading \n
    put(el, t.slice(0, s) + t.slice(e), Math.min(s, Math.max(0, t.length - (e - s) - 1)));
    return;
  }

  // operator pending → motion resolves it
  if (st.op === "d" || st.op === "c" || st.op === "y") {
    const q = motion(t, p, k, n, !!st.count);
    st.count = "";
    const op = st.op;
    st.op = "";
    if (q < 0) return;
    let [s, e] = [Math.min(p, q), Math.max(p, q)];
    if (k === "e") e++; // e is inclusive
    if (k === "w" && op === "c") {
      // cw acts like ce (vim quirk)
      const we = wordEnd(t, p) + 1;
      if (we > s) e = we;
    }
    yank(t.slice(s, e), false);
    if (op === "y") {
      el.setSelectionRange(s, s);
      return;
    }
    put(el, t.slice(0, s) + t.slice(e), s);
    if (op === "c") setMode(doc, el, "insert");
    return;
  }

  switch (k) {
    case "i": setMode(doc, el, "insert"); break;
    case "a": el.setSelectionRange(Math.min(lineEnd(t, p), p + 1), Math.min(lineEnd(t, p), p + 1)); setMode(doc, el, "insert"); break;
    case "I": { const q = firstNonBlank(t, p); el.setSelectionRange(q, q); setMode(doc, el, "insert"); break; }
    case "A": { const q = lineEnd(t, p); el.setSelectionRange(q, q); setMode(doc, el, "insert"); break; }
    case "o": { const e = lineEnd(t, p); put(el, t.slice(0, e) + "\n" + t.slice(e), e + 1); setMode(doc, el, "insert"); break; }
    case "O": { const s = lineStart(t, p); put(el, t.slice(0, s) + "\n" + t.slice(s), s); setMode(doc, el, "insert"); break; }
    case "v": {
      const ps = pend(el);
      ps.anchor = p;
      ps.head = p;
      setMode(doc, el, "v");
      break;
    }
    case "V": {
      const ps = pend(el);
      ps.anchor = p;
      ps.head = p;
      setMode(doc, el, "V");
      const [vs, ve] = lineSpan(t, p, p);
      el.setSelectionRange(vs, ve);
      break;
    }
    case "s": {
      const e = Math.min(lineEnd(t, p), p + n);
      if (e > p) {
        yank(t.slice(p, e), false);
        put(el, t.slice(0, p) + t.slice(e), p);
      }
      setMode(doc, el, "insert");
      break;
    }
    case "x": { const e = Math.min(lineEnd(t, p), p + n); if (e > p) { yank(t.slice(p, e), false); put(el, t.slice(0, p) + t.slice(e), p); } break; }
    case "D": { const e = lineEnd(t, p); yank(t.slice(p, e), false); put(el, t.slice(0, p) + t.slice(e), Math.max(lineStart(t, p), p - (e > p ? 0 : 1))); break; }
    case "C": { const e = lineEnd(t, p); yank(t.slice(p, e), false); put(el, t.slice(0, p) + t.slice(e), p); setMode(doc, el, "insert"); break; }
    case "d": case "c": case "y": st.op = k; return;
    case "g": st.op = "g"; return;
    case "p": {
      if (regLine) { const e = lineEnd(t, p); const at = e === t.length ? e : e + 1; const ins = e === t.length ? "\n" + reg.replace(/\n$/, "") : reg; put(el, t.slice(0, at) + ins + t.slice(at), at + (e === t.length ? 1 : 0)); }
      else put(el, t.slice(0, p + (t.length ? 1 : 0)) + reg + t.slice(p + (t.length ? 1 : 0)), p + reg.length);
      break;
    }
    case "P": {
      if (regLine) { const s = lineStart(t, p); put(el, t.slice(0, s) + reg + t.slice(s), s); }
      else { put(el, t.slice(0, p) + reg + t.slice(p), p + Math.max(0, reg.length - 1)); }
      break;
    }
    case "u": clickKbd(doc, "mod+z"); break;
    case "Escape": break;
    default: {
      const q = motion(t, p, k, n, !!st.count);
      if (q >= 0) el.setSelectionRange(q, q);
    }
  }
  st.count = "";
}

function visualKey(doc: Document, el: TA, k: string, line: boolean): void {
  const t = el.value;
  const st = pend(el);
  const n = st.count ? parseInt(st.count, 10) : 1;
  if (/^[1-9]$/.test(k) || (k === "0" && st.count)) {
    st.count += k;
    return;
  }
  const head = st.head;
  if (k === "Escape" || k === "v" || k === "V") {
    setMode(doc, el, "normal");
    el.setSelectionRange(head, head);
    st.count = "";
    return;
  }
  if (k === "d" || k === "c" || k === "y" || k === "x" || k === "s") {
    let s: number, e: number;
    if (line) [s, e] = lineSpan(t, st.anchor, head);
    else { s = Math.min(st.anchor, head); e = Math.max(st.anchor, head) + 1; e = Math.min(e, t.length); }
    yank(t.slice(s, e), line);
    setMode(doc, el, "normal");
    st.count = "";
    if (k === "y") { el.setSelectionRange(s, s); return; }
    if (line && k !== "c" && k !== "s" && e === t.length && s > 0) s--; // EOF eats the leading \n
    put(el, t.slice(0, s) + t.slice(e), Math.min(s, Math.max(0, t.slice(0, s).length + t.slice(e).length - 1)));
    if (k === "c" || k === "s") setMode(doc, el, "insert");
    return;
  }
  const q = motion(t, head, k, n, !!st.count);
  st.count = "";
  if (q < 0) return;
  st.head = q;
  if (line) {
    const [s, e] = lineSpan(t, st.anchor, q);
    el.setSelectionRange(s, e);
  } else el.setSelectionRange(Math.min(st.anchor, q), Math.max(st.anchor, q));
}

/** Install on a document (exported so tests and the sandboxed E2E harness can
 * pass their own; the auto-install below covers the browser). */
export function i(doc: Document): void {
  doc.addEventListener(
    "keydown",
    (e) => {
      const el = e.target as TA;
      if (!el || !el.getAttribute) return;
      const mode = el.getAttribute("data-vim") as Mode | null;
      if (mode === null) return;
      const ke = e as KeyboardEvent;
      const k = ke.key;
      const mod = ke.metaKey || ke.ctrlKey;
      if (mode === "insert") {
        if (k === "Escape") {
          e.preventDefault();
          setMode(doc, el, "normal");
        }
        return; // insert mode is the ordinary editor
      }
      if (mod) {
        if (k === "r") {
          e.preventDefault();
          clickKbd(doc, "mod+shift+z");
        }
        return;
      }
      if (k.length > 1 && k !== "Escape") return; // arrows etc. stay native
      e.preventDefault();
      if (mode === "normal") normalKey(doc, el, k);
      else visualKey(doc, el, k, mode === "V");
    },
    true, // capture: ahead of the core data-kbd hook and delegation
  );
}

if (typeof document !== "undefined") i(document);
