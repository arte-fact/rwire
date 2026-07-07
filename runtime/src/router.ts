// Router glue: intercept `a[data-route]` clicks into route messages over the
// socket, data-copy clipboard affordance, Enter-submits-form behavior for
// `[data-enter-submit]` fields, and popstate → route re-request.

import { st } from "./state.ts";
import { sh } from "./hash.ts";
import { bx, bj } from "./connect.ts";

export function installRouter(): void {
  document.addEventListener("click", (e) => {
    const a = (e.target as Element).closest("a[data-route]");
    if (a) {
      if (st.w && st.w.readyState === 1) {
        e.preventDefault();
        const h = a.getAttribute("href")!;
        history.pushState(null, "", bj(h));
        st.w.send("R" + h);
        const hs = h.indexOf("#");
        if (hs >= 0) sh(h.slice(hs));
        else scrollTo(0, 0);
      } /* socket not open (reconnecting): fall through to a normal full navigation instead of a dead click */
    }
    const b = (e.target as Element).closest("[data-copy]") as HTMLElement | null;
    if (b) {
      navigator.clipboard.writeText(b.dataset.copy!);
      b.classList.add("copied");
      setTimeout(() => b.classList.remove("copied"), 2000);
    }
  });
  document.addEventListener("keydown", (e) => {
    if (
      e.key === "Enter" &&
      !e.shiftKey &&
      !e.isComposing &&
      (e.target as Element).matches &&
      (e.target as Element).matches("[data-enter-submit]")
    ) {
      e.preventDefault();
      const f = (e.target as Element).closest("form");
      if (f) f.requestSubmit();
    }
    // Generic shortcut hook: [data-kbd="combo"] elements are clicked when
    // their combo is pressed (combo = "mod+"?"shift+"?key, e.g. "mod+s",
    // "mod+shift+z", "f2", "escape", "delete"). Bare-key combos are ignored
    // while typing in a field — except escape, which cancels.
    const ke = e as KeyboardEvent;
    const mod = ke.metaKey || ke.ctrlKey;
    const key = ke.key.toLowerCase();
    const combo = (mod ? "mod+" : "") + (ke.shiftKey ? "shift+" : "") + key;
    const t = document.querySelector('[data-kbd="' + combo + '"]') as HTMLElement | null;
    if (t) {
      const tgt = e.target as Element;
      // A vim-moded editor owns its keys outside insert (Esc = leave mode,
      // not cancel-prompt); the vim extension handles them in capture phase.
      const vim = tgt.getAttribute && tgt.getAttribute("data-vim");
      if (vim && vim !== "insert") return;
      const tag = tgt.tagName;
      const field = tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
      if (mod || !field || key === "escape") {
        e.preventDefault();
        t.click();
      }
    }
  });
  // Editor overlay echo: a [data-echo] field mirrors its value into the
  // named underlay on every keystroke, so transparent-ink typing is never
  // invisible; the server morph restores syntax colors moments later.
  document.addEventListener("input", (e) => {
    const t = e.target as HTMLTextAreaElement;
    const id = t.getAttribute && t.getAttribute("data-echo");
    if (id) {
      const u = document.getElementById(id);
      if (u) u.textContent = t.value;
    }
  });

  // Tooltip escape hatch: absolute popups clip inside overflow ancestors, so
  // on hover we re-anchor the [data-tip] popup as position:fixed from the
  // trigger's viewport rect (placement letter in data-tt: t/b/l/r).
  document.addEventListener("mouseover", (e) => {
    const c = (e.target as Element).closest?.("[data-tt]") as HTMLElement | null;
    if (!c || !c.getBoundingClientRect) return;
    const p = c.querySelector("[data-tip]") as HTMLElement | null;
    if (!p) return;
    const r = c.getBoundingClientRect();
    const cx = r.left + r.width / 2, cy = r.top + r.height / 2;
    const m: Record<string, [number, number, string]> = {
      t: [cx, r.top - 6, "-50%,-100%"],
      b: [cx, r.bottom + 6, "-50%,0"],
      l: [r.left - 6, cy, "-100%,-50%"],
      r: [r.right + 6, cy, "0,-50%"],
    };
    const [x, y, tr] = m[c.getAttribute("data-tt")!] || m.b;
    p.setAttribute("style", "position:fixed;left:" + x + "px;top:" + y + "px;transform:translate(" + tr + ")");
  });

  window.addEventListener("popstate", () => {
    st.w!.send("R" + bx(location.pathname));
    if (location.hash) sh(location.hash);
    else scrollTo(0, 0);
  });
}
