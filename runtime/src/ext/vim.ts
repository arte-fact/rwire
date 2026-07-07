// Vim extension — modal text editing for rwire editors.
//
// Lazy-loaded runtime extension (docs/vim-mode-design.md): NOT part of the
// core capsule. Speaks to the rest of the system only through the DOM
// contract — reads/writes the `data-vim` mode attribute, updates the
// `[data-vim-chip]` label, moves the native caret/selection, and dispatches
// synthetic bubbling `input` events after every mutation so the delegated
// dispatcher, overlay echo, dirty diff, and autosave all flow unchanged.
// Undo/redo delegate to the server history by clicking the [data-kbd]
// elements. Pending counts/operators/objects and the unnamed register are
// input-method state and live here by design. Motions for TEXT EDITING only:
// no ex commands, ever.
//
// v5 scope: normal/insert/v/V · h j k l 0 ^ $ w b e W B E gg G { } % ·
// f F t T ; , · counts · d c y (+motions/objects/doubling) · text objects
// iw aw iW aW i"/a" i'/a' i`/a` and bracket pairs · x s D C p P o O a A I ·
// r J ~ · >> << and visual > < · u/Ctrl-R via server history.

/** Version stamp logged by the loader — bump with every engine change so a
 * stale cached/embedded module is instantly visible in the console. */
export const v = 5;

type Mode = "normal" | "insert" | "v" | "V";
interface Pend {
  count: string;
  op: "" | "d" | "c" | "y" | "g" | ">" | "<";
  /** Object prefix awaited after an operator or in visual: "i" | "a". */
  obj: "" | "i" | "a";
  /** Two-key pending: f F t T (seek) or r (replace). */
  seek: "" | "f" | "F" | "t" | "T" | "r";
  anchor: number; // visual-mode anchor
  head: number; // visual-mode cursor (selection ends can't encode it in V)
}

const P = new WeakMap<Element, Pend>();
let reg = "";
let regLine = false;
let lastSeek: { k: string; ch: string } | null = null;

const pend = (el: Element): Pend => {
  let p = P.get(el);
  if (!p) {
    p = { count: "", op: "", obj: "", seek: "", anchor: 0, head: 0 };
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
const bigCls = (c: string): number => (c === " " || c === "\t" || c === "\n" ? 0 : 1);

function wordFwd(t: string, p: number, cl: (c: string) => number): number {
  if (p >= t.length) return p;
  const k = cl(t[p]);
  if (k !== 0) while (p < t.length && cl(t[p]) === k) p++;
  while (p < t.length && cl(t[p]) === 0) p++;
  return p;
}
function wordBack(t: string, p: number, cl: (c: string) => number): number {
  if (p <= 0) return 0;
  p--;
  while (p > 0 && cl(t[p]) === 0) p--;
  const k = cl(t[p]);
  while (p > 0 && cl(t[p - 1]) === k) p--;
  return p;
}
function wordEnd(t: string, p: number, cl: (c: string) => number): number {
  p++;
  while (p < t.length && cl(t[p]) === 0) p++;
  if (p >= t.length) return t.length ? t.length - 1 : 0;
  const k = cl(t[p]);
  while (p + 1 < t.length && cl(t[p + 1]) === k) p++;
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
/** Next/previous empty line (paragraph motion). */
function paraMove(t: string, p: number, dir: 1 | -1): number {
  let i = p;
  if (dir === 1) {
    i = lineEnd(t, i);
    while (i < t.length) {
      const ns = i + 1;
      const ne = lineEnd(t, ns);
      if (ns === ne) return ns; // empty line
      i = ne;
    }
    return t.length;
  }
  i = lineStart(t, i);
  while (i > 0) {
    const pe = i - 1;
    const ps = lineStart(t, pe);
    if (ps === pe) return ps;
    i = ps;
  }
  return 0;
}
const OPENS = "([{<";
const CLOSES = ")]}>";
/** Matching-bracket jump from the first bracket at/after p on the line. */
function matchBracket(t: string, p: number): number {
  const le = lineEnd(t, p);
  let i = p;
  while (i < le && OPENS.indexOf(t[i]) === -1 && CLOSES.indexOf(t[i]) === -1) i++;
  if (i >= le) return -1;
  const c = t[i];
  const oi = OPENS.indexOf(c);
  if (oi !== -1) {
    const close = CLOSES[oi];
    let depth = 0;
    for (let j = i; j < t.length; j++) {
      if (t[j] === c) depth++;
      else if (t[j] === close && --depth === 0) return j;
    }
    return -1;
  }
  const ci = CLOSES.indexOf(c);
  const open = OPENS[ci];
  let depth = 0;
  for (let j = i; j >= 0; j--) {
    if (t[j] === c) depth++;
    else if (t[j] === open && --depth === 0) return j;
  }
  return -1;
}
/** f/F/t/T target position (caret lands per kind), or -1. */
function seekPos(t: string, p: number, kind: string, ch: string, n: number): number {
  const ls = lineStart(t, p);
  const le = lineEnd(t, p);
  let q = p;
  for (let i = 0; i < n; i++) {
    if (kind === "f" || kind === "t") {
      const from = q + (kind === "t" && i === 0 && t[q + 1] === ch ? 2 : 1);
      const hit = t.indexOf(ch, from);
      if (hit === -1 || hit > le) return -1;
      q = hit;
    } else {
      const from = q - (kind === "T" && i === 0 && t[q - 1] === ch ? 2 : 1);
      const hit = t.lastIndexOf(ch, from);
      if (hit === -1 || hit < ls) return -1;
      q = hit;
    }
  }
  if (kind === "t") return q - 1;
  if (kind === "T") return q + 1;
  return q;
}

// ------------------------------------------------------------ text objects
const PAIRS: Record<string, [string, string]> = {
  "(": ["(", ")"],
  ")": ["(", ")"],
  b: ["(", ")"],
  "{": ["{", "}"],
  "}": ["{", "}"],
  B: ["{", "}"],
  "[": ["[", "]"],
  "]": ["[", "]"],
  "<": ["<", ">"],
  ">": ["<", ">"],
};

/** Resolve a text object at p: [start, endExcl) or null. */
function textObject(t: string, p: number, around: boolean, key: string): [number, number] | null {
  if (key === "w" || key === "W") {
    const cl = key === "w" ? cls : bigCls;
    if (p >= t.length) return null;
    const k = cl(t[p]);
    let s = p;
    let e = p;
    while (s > 0 && cl(t[s - 1]) === k && t[s - 1] !== "\n") s--;
    while (e < t.length && cl(t[e]) === k && t[e] !== "\n") e++;
    if (around) {
      let e2 = e;
      while (e2 < t.length && (t[e2] === " " || t[e2] === "\t")) e2++;
      if (e2 > e) e = e2;
      else while (s > 0 && (t[s - 1] === " " || t[s - 1] === "\t")) s--;
    }
    return s < e ? [s, e] : null;
  }
  if (key === '"' || key === "'" || key === "`") {
    // quote pair on the caret's line: enclosing, else the next pair after p
    const ls = lineStart(t, p);
    const le = lineEnd(t, p);
    const hits: number[] = [];
    for (let i = ls; i < le; i++) if (t[i] === key && t[i - 1] !== "\\") hits.push(i);
    for (let i = 0; i + 1 < hits.length; i += 2) {
      const a = hits[i];
      const b = hits[i + 1];
      if (p <= b) return around ? [a, b + 1] : [a + 1, b];
    }
    return null;
  }
  const pair = PAIRS[key];
  if (pair) {
    const open = pair[0];
    const close = pair[1];
    // scan back for the unmatched opener enclosing p
    let depth = 0;
    let s = -1;
    for (let i = p; i >= 0; i--) {
      if (t[i] === close && i !== p) depth++;
      else if (t[i] === open) {
        if (depth === 0) {
          s = i;
          break;
        }
        depth--;
      }
    }
    if (s === -1) return null;
    depth = 0;
    for (let j = s; j < t.length; j++) {
      if (t[j] === open) depth++;
      else if (t[j] === close && --depth === 0) return around ? [s, j + 1] : [s + 1, j];
    }
    return null;
  }
  return null;
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

/** Single-key motions valid in normal+visual. Returns new pos or -1. */
function motion(t: string, p: number, k: string, n: number, hasCount: boolean): number {
  let q = p;
  for (let i = 0; i < n; i++) {
    switch (k) {
      case "h": q = Math.max(lineStart(t, q), q - 1); break;
      case "l": q = Math.min(lineEnd(t, q), q + 1); break;
      case "j": q = lineMove(t, q, 1); break;
      case "k": q = lineMove(t, q, -1); break;
      case "w": q = wordFwd(t, q, cls); break;
      case "W": q = wordFwd(t, q, bigCls); break;
      case "b": q = wordBack(t, q, cls); break;
      case "B": q = wordBack(t, q, bigCls); break;
      case "e": q = wordEnd(t, q, cls); break;
      case "E": q = wordEnd(t, q, bigCls); break;
      case "{": q = paraMove(t, q, -1); break;
      case "}": q = paraMove(t, q, 1); break;
      case "0": return lineStart(t, p);
      case "^": return firstNonBlank(t, p);
      case "$":
        q = lineEnd(t, q);
        if (i < n - 1 && q < t.length) q++;
        break;
      case "%": {
        const m = matchBracket(t, q);
        return m === -1 ? -1 : m;
      }
      case "G":
        return hasCount ? lineForCount(t, n) : firstNonBlank(t, lineStart(t, t.length));
      default:
        return -1;
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
/** Motions whose operator range includes the landing character. */
const INCLUSIVE = ["e", "E", "%"];

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

/** Indent/dedent whole lines covering [a,b]; returns the new text. */
function shiftLines(t: string, a: number, b: number, out: boolean): string {
  const [s, e] = lineSpan(t, a, b);
  const block = t.slice(s, e);
  const trailingNl = block.endsWith("\n");
  const lines = (trailingNl ? block.slice(0, -1) : block).split("\n");
  const shifted = lines
    .map((l) => {
      if (!out) return l.length ? "\t" + l : l;
      if (l[0] === "\t") return l.slice(1);
      let cut = 0;
      while (cut < 4 && l[cut] === " ") cut++;
      return l.slice(cut);
    })
    .join("\n");
  return t.slice(0, s) + shifted + (trailingNl ? "\n" : "") + t.slice(e);
}

/** Apply operator op over [s,e) charwise. */
function opRange(doc: Document, el: TA, op: string, s: number, e: number): void {
  const t = el.value;
  yank(t.slice(s, e), false);
  if (op === "y") {
    el.setSelectionRange(s, s);
    return;
  }
  put(el, t.slice(0, s) + t.slice(e), s);
  if (op === "c") setMode(doc, el, "insert");
}

const reverseSeek = (k: string): string =>
  k === "f" ? "F" : k === "F" ? "f" : k === "t" ? "T" : "t";

/** Operator over a seek motion: f/t include through the landing char forward,
 * F/T take from the landing char back to the caret. */
function applySeekOp(doc: Document, el: TA, p: number, q: number, kind: string, op: string): void {
  let s: number;
  let e: number;
  if (kind === "f" || kind === "t") {
    s = p;
    e = q + 1;
  } else {
    s = q;
    e = p;
  }
  if (s < e) opRange(doc, el, op, s, e);
}

// ------------------------------------------------------------------ keys
function normalKey(doc: Document, el: TA, k: string): void {
  const t = el.value;
  const p = el.selectionStart;
  const st = pend(el);
  const n = st.count ? parseInt(st.count, 10) : 1;
  const hadCount = !!st.count;

  // two-key pendings resolve first: f F t T r take ANY next char
  if (st.seek) {
    const kind = st.seek;
    st.seek = "";
    st.count = "";
    if (k === "Escape") {
      st.op = "";
      return;
    }
    if (kind === "r") {
      const e = Math.min(lineEnd(t, p), p + n);
      if (e > p) put(el, t.slice(0, p) + k.repeat(e - p) + t.slice(e), Math.max(p, e - 1));
      return;
    }
    lastSeek = { k: kind, ch: k };
    const q = seekPos(t, p, kind, k, n);
    if (q === -1) {
      st.op = "";
      return;
    }
    if (st.op === "d" || st.op === "c" || st.op === "y") {
      const op = st.op;
      st.op = "";
      applySeekOp(doc, el, p, q, kind, op);
      return;
    }
    el.setSelectionRange(q, q);
    return;
  }

  if (k === "Escape") {
    st.op = "";
    st.obj = "";
    st.count = "";
    return;
  }

  // counts (0 is a motion when no count pending)
  if (/^[1-9]$/.test(k) || (k === "0" && st.count)) {
    st.count += k;
    return;
  }

  // gg
  if (st.op === "g") {
    st.op = "";
    if (k === "g") {
      const q = hadCount ? lineForCount(t, n) : firstNonBlank(t, 0);
      st.count = "";
      el.setSelectionRange(q, q);
    }
    return;
  }

  // >> / <<
  if (st.op === ">" || st.op === "<") {
    const op = st.op;
    st.op = "";
    st.count = "";
    if (k === op) {
      let [s2, e2] = lineSpan(t, p, p);
      for (let i = 1; i < n; i++) e2 = lineSpan(t, e2, e2)[1];
      put(el, shiftLines(t, s2, Math.max(s2, e2 - 1), op === "<"), p);
    }
    return;
  }

  // operator pending
  if (st.op === "d" || st.op === "c" || st.op === "y") {
    const op = st.op;
    if (st.obj) {
      // diw / ca( ...
      const around = st.obj === "a";
      st.obj = "";
      st.op = "";
      st.count = "";
      const r = textObject(t, p, around, k);
      if (r) opRange(doc, el, op, r[0], r[1]);
      return;
    }
    if (k === "i" || k === "a") {
      st.obj = k;
      return;
    }
    if (k === "f" || k === "F" || k === "t" || k === "T") {
      st.seek = k;
      return;
    }
    if (k === ";" || k === ",") {
      st.op = "";
      st.count = "";
      if (!lastSeek) return;
      const kind = k === ";" ? lastSeek.k : reverseSeek(lastSeek.k);
      const q = seekPos(t, p, kind, lastSeek.ch, n);
      if (q !== -1) applySeekOp(doc, el, p, q, kind, op);
      return;
    }
    if (k === op) {
      // dd / yy / cc — n lines
      let [s, e] = lineSpan(t, p, p);
      for (let i = 1; i < n; i++) e = lineSpan(t, e, e)[1];
      yank(t.slice(s, e), true);
      st.count = "";
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
    const q = motion(t, p, k, n, hadCount);
    st.count = "";
    st.op = "";
    if (q < 0) return;
    let s = Math.min(p, q);
    let e = Math.max(p, q);
    if (INCLUSIVE.indexOf(k) !== -1) e++;
    if (k === "w" && op === "c") {
      // cw acts like ce (vim quirk)
      const we = wordEnd(t, p, cls) + 1;
      if (we > s) e = we;
    }
    opRange(doc, el, op, s, e);
    return;
  }

  switch (k) {
    case "i":
      setMode(doc, el, "insert");
      break;
    case "a": {
      const q = Math.min(lineEnd(t, p), p + 1);
      el.setSelectionRange(q, q);
      setMode(doc, el, "insert");
      break;
    }
    case "I": {
      const q = firstNonBlank(t, p);
      el.setSelectionRange(q, q);
      setMode(doc, el, "insert");
      break;
    }
    case "A": {
      const q = lineEnd(t, p);
      el.setSelectionRange(q, q);
      setMode(doc, el, "insert");
      break;
    }
    case "o": {
      const e = lineEnd(t, p);
      put(el, t.slice(0, e) + "\n" + t.slice(e), e + 1);
      setMode(doc, el, "insert");
      break;
    }
    case "O": {
      const s = lineStart(t, p);
      put(el, t.slice(0, s) + "\n" + t.slice(s), s);
      setMode(doc, el, "insert");
      break;
    }
    case "v": {
      st.anchor = p;
      st.head = p;
      setMode(doc, el, "v");
      el.setSelectionRange(p, Math.min(p + 1, t.length));
      break;
    }
    case "V": {
      st.anchor = p;
      st.head = p;
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
    case "x": {
      const e = Math.min(lineEnd(t, p), p + n);
      if (e > p) {
        yank(t.slice(p, e), false);
        put(el, t.slice(0, p) + t.slice(e), p);
      }
      break;
    }
    case "r":
      st.seek = "r";
      return; // keep the count for r
    case "~": {
      const e = Math.min(lineEnd(t, p), p + n);
      if (e > p) {
        const flipped = t
          .slice(p, e)
          .replace(/./g, (c) => (c === c.toLowerCase() ? c.toUpperCase() : c.toLowerCase()));
        put(el, t.slice(0, p) + flipped + t.slice(e), Math.min(e, Math.max(p, lineEnd(t, p) - 1)));
      }
      break;
    }
    case "J": {
      let txt = t;
      let caret = p;
      const joins = Math.max(1, n - 1);
      for (let i = 0; i < joins; i++) {
        const e = lineEnd(txt, caret);
        if (e === txt.length) break;
        let ns = e + 1;
        while (ns < txt.length && (txt[ns] === " " || txt[ns] === "\t")) ns++;
        txt = txt.slice(0, e) + " " + txt.slice(ns);
        caret = e;
      }
      if (txt !== t) put(el, txt, caret);
      break;
    }
    case "D": {
      const e = lineEnd(t, p);
      yank(t.slice(p, e), false);
      put(el, t.slice(0, p) + t.slice(e), Math.max(lineStart(t, p), p - (e > p ? 0 : 1)));
      break;
    }
    case "C": {
      const e = lineEnd(t, p);
      yank(t.slice(p, e), false);
      put(el, t.slice(0, p) + t.slice(e), p);
      setMode(doc, el, "insert");
      break;
    }
    case "d":
    case "c":
    case "y":
      st.op = k;
      return;
    case ">":
    case "<":
      st.op = k;
      return;
    case "g":
      st.op = "g";
      return;
    case "f":
    case "F":
    case "t":
    case "T":
      st.seek = k;
      return; // keep the count for the seek
    case ";":
    case ",": {
      if (!lastSeek) break;
      const kind = k === ";" ? lastSeek.k : reverseSeek(lastSeek.k);
      const q = seekPos(t, p, kind, lastSeek.ch, n);
      if (q !== -1) el.setSelectionRange(q, q);
      break;
    }
    case "p": {
      if (regLine) {
        const e = lineEnd(t, p);
        const at = e === t.length ? e : e + 1;
        const ins = e === t.length ? "\n" + reg.replace(/\n$/, "") : reg;
        put(el, t.slice(0, at) + ins + t.slice(at), at + (e === t.length ? 1 : 0));
      } else
        put(
          el,
          t.slice(0, p + (t.length ? 1 : 0)) + reg + t.slice(p + (t.length ? 1 : 0)),
          p + reg.length,
        );
      break;
    }
    case "P": {
      if (regLine) {
        const s = lineStart(t, p);
        put(el, t.slice(0, s) + reg + t.slice(s), s);
      } else {
        put(el, t.slice(0, p) + reg + t.slice(p), p + Math.max(0, reg.length - 1));
      }
      break;
    }
    case "u":
      clickKbd(doc, "mod+z");
      break;
    default: {
      const q = motion(t, p, k, n, hadCount);
      if (q >= 0) el.setSelectionRange(q, q);
    }
  }
  st.count = "";
}

function visualKey(doc: Document, el: TA, k: string, line: boolean): void {
  const t = el.value;
  const st = pend(el);
  const n = st.count ? parseInt(st.count, 10) : 1;

  // pending seek char
  if (st.seek) {
    const kind = st.seek;
    st.seek = "";
    st.count = "";
    if (k === "Escape" || kind === "r") return;
    lastSeek = { k: kind, ch: k };
    const q = seekPos(t, st.head, kind, k, n);
    if (q !== -1) extendTo(el, t, st, q, line);
    return;
  }
  if (/^[1-9]$/.test(k) || (k === "0" && st.count)) {
    st.count += k;
    return;
  }
  const head = st.head;

  // object selection: viw / va( ...
  if (st.obj) {
    const around = st.obj === "a";
    st.obj = "";
    st.count = "";
    const r = textObject(t, head, around, k);
    if (r) {
      st.anchor = r[0];
      extendTo(el, t, st, Math.max(r[0], r[1] - 1), line);
    }
    return;
  }
  if (!line && (k === "i" || k === "a")) {
    st.obj = k;
    return;
  }

  if (k === "Escape" || k === "v" || k === "V") {
    setMode(doc, el, "normal");
    el.setSelectionRange(head, head);
    st.count = "";
    return;
  }
  if (k === "f" || k === "F" || k === "t" || k === "T") {
    st.seek = k;
    return;
  }
  if (k === ";" || k === ",") {
    st.count = "";
    if (!lastSeek) return;
    const kind = k === ";" ? lastSeek.k : reverseSeek(lastSeek.k);
    const q = seekPos(t, head, kind, lastSeek.ch, n);
    if (q !== -1) extendTo(el, t, st, q, line);
    return;
  }
  if (k === ">" || k === "<") {
    const s = Math.min(st.anchor, head);
    const e = Math.max(st.anchor, head);
    setMode(doc, el, "normal");
    st.count = "";
    put(el, shiftLines(t, s, e, k === "<"), lineStart(t, s));
    return;
  }
  if (k === "d" || k === "c" || k === "y" || k === "x" || k === "s") {
    let s: number;
    let e: number;
    if (line) [s, e] = lineSpan(t, st.anchor, head);
    else {
      s = Math.min(st.anchor, head);
      e = Math.min(Math.max(st.anchor, head) + 1, t.length);
    }
    yank(t.slice(s, e), line);
    setMode(doc, el, "normal");
    st.count = "";
    if (k === "y") {
      el.setSelectionRange(s, s);
      return;
    }
    if (line && k !== "c" && k !== "s" && e === t.length && s > 0) s--; // EOF eats the leading \n
    put(el, t.slice(0, s) + t.slice(e), Math.min(s, Math.max(0, t.slice(0, s).length + t.slice(e).length - 1)));
    if (k === "c" || k === "s") setMode(doc, el, "insert");
    return;
  }
  const q = motion(t, head, k, n, !!st.count);
  st.count = "";
  if (q < 0) return;
  extendTo(el, t, st, q, line);
}

function extendTo(el: TA, t: string, st: Pend, q: number, line: boolean): void {
  st.head = q;
  if (line) {
    const [s, e] = lineSpan(t, st.anchor, q);
    el.setSelectionRange(s, e);
  } else
    el.setSelectionRange(Math.min(st.anchor, q), Math.min(t.length, Math.max(st.anchor, q) + 1));
}

const installed = new WeakSet<Document>();

/** Install on a document (exported so tests and the sandboxed E2E harness can
 * pass their own; the auto-install below covers the browser). IDEMPOTENT per
 * document: in a real browser BOTH the import side effect and the loader's
 * m.i(document) run — a double install would handle every keydown twice,
 * making v enter AND exit visual within one keypress. */
export function i(doc: Document): void {
  if (installed.has(doc)) return;
  installed.add(doc);
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
