// Vim extension — M1 skeleton (mode plumbing; the engine lands in M2).
//
// Lazy-loaded runtime extension: NOT part of the core capsule. The server
// hints it via MOD_DEF; the core loader dynamic-imports this module and calls
// `i(document)`. The module is self-contained: it speaks to the rest of the
// system only through the DOM contract — reads/writes the `data-vim` mode
// attribute, updates the `[data-vim-chip]` label, and dispatches synthetic
// bubbling `input` events that the delegated dispatcher and overlay echo
// already understand. Vim pending state (counts, operators, registers) is
// input-method state and lives here, client-side, by design
// (docs/vim-mode-design.md rev.4).

type Mode = "normal" | "insert" | "v" | "V";

function setMode(doc: Document, el: HTMLTextAreaElement, m: Mode): void {
  el.setAttribute("data-vim", m);
  const chip = doc.querySelector("[data-vim-chip]");
  if (chip) {
    chip.textContent = m === "v" ? "VISUAL" : m === "V" ? "V-LINE" : m.toUpperCase();
    chip.setAttribute("data-vim-mode", m);
  }
}

/** Install on a document (exported so tests and the sandboxed E2E harness can
 * pass their own; the auto-install below covers the browser). */
export function i(doc: Document): void {
  doc.addEventListener(
    "keydown",
    (e) => {
      const t = e.target as HTMLTextAreaElement;
      if (!t || !t.getAttribute) return;
      const mode = t.getAttribute("data-vim") as Mode | null;
      if (mode === null) return;
      const k = (e as KeyboardEvent).key;
      const mod = (e as KeyboardEvent).metaKey || (e as KeyboardEvent).ctrlKey;
      if (mode === "insert") {
        if (k === "Escape") {
          e.preventDefault();
          setMode(doc, t, "normal");
        }
        return; // insert mode is the ordinary editor
      }
      // normal / visual: the engine (M2) interprets; the skeleton handles
      // mode entry/exit and swallows unmodified printable keys so normal
      // mode never types.
      if (mod) return;
      if (k === "i") {
        e.preventDefault();
        setMode(doc, t, "insert");
        return;
      }
      if (k === "Escape") {
        e.preventDefault();
        setMode(doc, t, "normal");
        return;
      }
      if (k === "v") {
        e.preventDefault();
        setMode(doc, t, mode === "v" ? "normal" : "v");
        return;
      }
      if (k === "V") {
        e.preventDefault();
        setMode(doc, t, mode === "V" ? "normal" : "V");
        return;
      }
      if (k.length === 1) e.preventDefault(); // swallow printables in normal/visual
    },
    true, // capture: run before the core data-kbd hook and any delegation
  );
}

if (typeof document !== "undefined") i(document);
