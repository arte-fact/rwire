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
  });
  window.addEventListener("popstate", () => {
    st.w!.send("R" + bx(location.pathname));
    if (location.hash) sh(location.hash);
    else scrollTo(0, 0);
  });
}
