// The opcode executor: one pass over a server message, building/patching DOM
// via a ref array. Transcribed 1:1 from the original hand-minified x() —
// behavior-identical is the bar, including the null-guards (a missing target
// becomes a detached no-op, never an aborted update) and the exact
// PARSE ERROR / Unknown opcode console formats the wire harness greps for.

import * as OP from "./opcodes.ts";
import { st, E, V, P, Y, AT, AV, SE, A, type RwEl } from "./state.ts";
import { rv } from "./varint.ts";
import { sa } from "./sanitize.ts";
import { sep } from "./events.ts";
import { fm } from "./morph.ts";
import { BL, xi } from "./bind.ts";
import { fl2, fb2, sl2, sb2, uf2, us2 } from "./actions.ts";
import { bind } from "./delegate.ts";

/** Extensions already imported (once per page). */
const ext = new Set<string>();
const TD = new TextDecoder();

export function x(d: Uint8Array): void {
  const r: RwEl[] = [];
  let i = 0,
    _oc = 0;
  // Focus snapshot: restored at BATCH_END so updates never eat the caret.
  const ae = document.activeElement as RwEl | null,
    ai = ae && ae.id,
    ap = ae ? (ae as any).selectionStart : 0,
    aq = ae ? (ae as any).selectionEnd : 0,
    ax = ae && (ae.tagName === "INPUT" || ae.tagName === "TEXTAREA"),
    av = ax ? (ae as any).value : null;
  try {
    while (i < d.length) {
      const _p = i,
        o = d[i++];
      _oc++;
      if (o === OP.SYMBOLS) {
        let [n, l] = rv(d, i);
        i += l;
        st.sc = 0x80;
        while (n--) {
          const [sl, ll] = rv(d, i);
          i += ll;
          st.s[st.sc++] = TD.decode(d.slice(i, i + sl));
          i += sl;
        }
      } else if (o === OP.SYMBOLS_EXTEND) {
        let [n, l] = rv(d, i);
        i += l;
        const [si, sl] = rv(d, i);
        i += sl;
        st.sc = si;
        while (n--) {
          const [sl2c, ll] = rv(d, i);
          i += ll;
          st.s[st.sc++] = TD.decode(d.slice(i, i + sl2c));
          i += sl2c;
        }
      } else if (o === OP.WORD_TABLE) {
        let n = d[i++];
        st.wt = [];
        while (n--) {
          const [l, ll] = rv(d, i);
          i += ll;
          st.wt.push(TD.decode(d.slice(i, i + l)));
          i += l;
        }
      } else if (o === OP.GET_BY_ID) {
        const [k, l] = rv(d, i);
        i += l;
        // null-guard: a missing target must not throw and abort the rest of the update
        const el = document.getElementById(st.s[k]);
        r.push((el || document.createElement("div")) as RwEl);
      } else if (o === OP.CREATE) {
        const t = d[i++];
        r.push(
          (SE[t]
            ? document.createElementNS("http://www.w3.org/2000/svg", E[t] || "svg")
            : document.createElement(E[t] || "div")) as RwEl,
        );
      } else if (o === OP.CREATE_SYNCED) {
        const [id, l] = rv(d, i);
        i += l;
        const e = document.createElement("span");
        e.id = "__synced_" + id;
        r.push(e as RwEl);
      } else if (o === OP.GET_SYNCED) {
        const [id, l] = rv(d, i);
        i += l;
        // null-guard: ops on a pruned/missing region become detached no-ops instead of aborting the update
        r.push(
          (document.getElementById("__synced_" + id) ||
            document.createElement("span")) as RwEl,
        );
      } else if (o === OP.SET_TEXT) {
        const [f, fl] = rv(d, i);
        i += fl;
        const [k, l] = rv(d, i);
        i += l;
        r[f].textContent = st.s[k] || "";
      } else if (o === OP.SET_TEXT_WORDS) {
        const [f, fl] = rv(d, i);
        i += fl;
        let n = d[i++];
        const ws: string[] = [];
        while (n--) ws.push(st.wt[d[i++]] || "");
        r[f].textContent = ws.join(" ");
      } else if (o === OP.SET_TEXT_INT) {
        const [f, fl] = rv(d, i);
        i += fl;
        const [v, l] = rv(d, i);
        i += l;
        const n = (v >>> 1) ^ -(v & 1); // zigzag
        r[f].textContent = n.toString();
      } else if (o === OP.SET_KEY) {
        const [f, fl] = rv(d, i);
        i += fl;
        const [k, l] = rv(d, i);
        i += l;
        r[f].__k = k;
      } else if (o === OP.SET_CLASS) {
        const [f, fl] = rv(d, i);
        i += fl;
        const [k, l] = rv(d, i);
        i += l;
        r[f].className = st.s[k] || "";
      } else if (o === OP.SET_ATTR) {
        const [f, fl] = rv(d, i);
        i += fl;
        const [ak, al] = rv(d, i);
        i += al;
        const [vk, vl] = rv(d, i);
        i += vl;
        const an = A[ak] || st.s[ak] || "data";
        sa(r[f], an, st.s[vk] || "");
      } else if (o === OP.SET_ATTR_ENUM) {
        const [f, fl] = rv(d, i);
        i += fl;
        const k = d[i++],
          v = d[i++];
        r[f].setAttribute(AT[k] || "data", AV[v] || "");
      } else if (o === OP.SET_ATTR_BOOL) {
        const [f, fl] = rv(d, i);
        i += fl;
        const k = d[i++];
        r[f].setAttribute(AT[k] || "data", "");
      } else if (o === OP.SET_ATTR_KEY_SYM) {
        const [f, fl] = rv(d, i);
        i += fl;
        const k = d[i++],
          [v, l] = rv(d, i);
        i += l;
        sa(r[f], AT[k] || "data", st.s[v] || "");
      } else if (o === OP.SET_DATA) {
        const [f, fl] = rv(d, i);
        i += fl;
        const [kk, kl] = rv(d, i);
        i += kl;
        const [vk, vl] = rv(d, i);
        i += vl;
        r[f].dataset[st.s[kk] || ""] = st.s[vk] || "";
      } else if (o === OP.APPEND) {
        const [p, pl] = rv(d, i);
        i += pl;
        const [c, cl] = rv(d, i);
        i += cl;
        (p < 0xffff ? r[p] : (document.body as RwEl)).appendChild(r[c]);
      } else if (o === OP.CLEAR_CHILDREN) {
        const [f, fl] = rv(d, i);
        i += fl;
        fm();
        const lv = r[f];
        const shd = document.createElement(lv.tagName || "DIV");
        st.pm = { live: lv, shadow: shd };
        r[f] = shd as RwEl;
      } else if (o === OP.BIND_LOCAL) {
        const [f, fl] = rv(d, i);
        i += fl;
        const t = d[i++];
        const [h, hl] = rv(d, i);
        i += hl;
        BL(f, t, h, r);
      } else if (o === OP.BIND_REMOTE) {
        const [f, fl] = rv(d, i);
        i += fl;
        const t = d[i++];
        const [h, hl] = rv(d, i);
        i += hl;
        r[f].__hk = "r" + t + "_" + h;
        bind(r[f], V[t] || "click", { h, t, f });
      } else if (o === OP.BIND_DEBOUNCED) {
        const [f, fl] = rv(d, i);
        i += fl;
        const t = d[i++];
        const [h, hl] = rv(d, i);
        i += hl;
        const ms = (d[i++] << 8) | d[i++];
        r[f].__hk = "d" + t + "_" + h;
        bind(r[f], V[t] || "click", { h, t, f, ms });
      } else if (o === OP.BIND_REMOTE_PARAM) {
        const [f, fl] = rv(d, i);
        i += fl;
        const t = d[i++];
        const [h, hl] = rv(d, i);
        i += hl;
        const pl = d[i++],
          prm = d.slice(i, i + pl);
        i += pl;
        r[f].__hk = "p" + t + "_" + h + "_" + prm.join(",");
        bind(r[f], V[t] || "click", { h, t, f, prm });
      } else if (o === OP.INLINE_LOCAL || o === OP.DEF_HANDLER) {
        i = xi(d, i - 1);
      } else if (o === OP.ROUTE_PUSH) {
        const [k, l] = rv(d, i);
        i += l;
        history.pushState(null, "", st.s[k]);
      } else if (o === OP.ROUTE_REPLACE) {
        const [k, l] = rv(d, i);
        i += l;
        history.replaceState(null, "", st.s[k]);
      } else if (o === OP.ROUTE_PUSH_INLINE) {
        const [sl, ll] = rv(d, i);
        i += ll;
        const u = TD.decode(d.slice(i, i + sl));
        i += sl;
        history.pushState(null, "", u);
      } else if (o === OP.ROUTE_REPLACE_INLINE) {
        const [sl, ll] = rv(d, i);
        i += ll;
        const u = TD.decode(d.slice(i, i + sl));
        i += sl;
        history.replaceState(null, "", u);
      } else if (o === OP.STYLE_SET) {
        const [f, fl] = rv(d, i);
        i += fl;
        const [k, l] = rv(d, i);
        i += l;
        r[f].style.cssText = st.s[k] || "";
      } else if (o === OP.STYLE_UTIL) {
        const [f, fl] = rv(d, i);
        i += fl;
        const [u, l] = rv(d, i);
        i += l;
        r[f].classList.add("u" + u);
      } else if (o === OP.STYLE_PROP) {
        const [f, fl] = rv(d, i);
        i += fl;
        const p = d[i++],
          v = d[i++];
        (r[f].style as any)[P[p]] = Y[v];
      } else if (o === OP.STYLE_MULTI) {
        const [f, fl] = rv(d, i);
        i += fl;
        let n = d[i++];
        while (n--) {
          const [u, l] = rv(d, i);
          i += l;
          r[f].classList.add("u" + u);
        }
      } else if (o === OP.COMPOSITE_TABLE) {
        let [n, l] = rv(d, i);
        i += l;
        while (n--) {
          const [id, il] = rv(d, i);
          i += il;
          let c = d[i++];
          while (c--) {
            const [, ul] = rv(d, i);
            i += ul;
          }
          st.K[id] = "c" + id;
        }
      } else if (o === OP.STYLE_DEF) {
        let [n, l] = rv(d, i);
        i += l;
        if (!st.DS) {
          st.DS = document.createElement("style");
          document.head.appendChild(st.DS);
        }
        while (n--) {
          const [rl, ll] = rv(d, i);
          i += ll;
          const rule = TD.decode(d.slice(i, i + rl));
          i += rl;
          try {
            st.DS.sheet!.insertRule(rule, st.DS.sheet!.cssRules.length);
          } catch (_e) {
            st.DS.textContent += rule;
          }
        }
      } else if (o === OP.MAP_DEF) {
        let [n, nl] = rv(d, i);
        i += nl;
        while (n--) {
          const k = d[i++],
            c = d[i++],
            [l, ll] = rv(d, i);
          i += ll;
          const nm = TD.decode(d.slice(i, i + l));
          i += l;
          if (k === 6) {
            E[c] = nm;
            SE[c] = 1;
          } else
            (k === 0 ? E : k === 1 ? V : k === 2 ? AT : k === 3 ? AV : k === 4 ? P : Y)[
              c
            ] = nm;
        }
      } else if (o === OP.MOD_DEF) {
        // Lazy runtime extensions: import each named module once per page;
        // the module's exported i(document) installs it. __rwImport lets the
        // sandboxed E2E harness substitute a loader.
        let [mn, mnl] = rv(d, i);
        i += mnl;
        while (mn--) {
          const [l, ll] = rv(d, i);
          i += ll;
          const name = TD.decode(d.slice(i, i + l));
          i += l;
          if (!ext.has(name)) {
            ext.add(name);
            const imp = (globalThis as any).__rwImport || ((u: string) => import(u));
            const u = BASE + "/_rw/ext/" + name + ".js";
            imp(u)
              .then((m: any) => m.i && m.i(document))
              .catch(() => {
                // some page contexts reject dynamic import; a module script
                // tag still works (the ext self-installs via side effect)
                const sc = document.createElement("script");
                sc.setAttribute("type", "module");
                sc.setAttribute("src", u);
                document.head.appendChild(sc);
              });
          }
        }
      } else if (o === OP.STYLE_COMPOSITE) {
        const [f, fl] = rv(d, i);
        i += fl;
        const [id, l] = rv(d, i);
        i += l;
        r[f].classList.add(st.K[id] || "c" + id);
      } else if (o === OP.STYLE_PSEUDO) {
        const [f, fl] = rv(d, i);
        i += fl;
        const pc = d[i++];
        let n = d[i++];
        while (n--) {
          const [u, l] = rv(d, i);
          i += l;
          r[f].classList.add("h" + pc + "u" + u);
        }
      } else if (o === OP.STYLE_BREAKPOINT) {
        const [f, fl] = rv(d, i);
        i += fl;
        const bp = d[i++];
        let n = d[i++];
        while (n--) {
          const [u, l] = rv(d, i);
          i += l;
          r[f].classList.add("b" + bp + "u" + u);
        }
      } else if (o === OP.INIT_TARGET) {
        fl2[d[i]] = !!d[i + 1];
        i += 2;
      } else if (o === OP.BIND_TARGET) {
        const [f, l] = rv(d, i);
        i += l;
        const ti = d[i++];
        const [stc, sl] = rv(d, i);
        i += sl;
        const inv = d[i++];
        (fb2[ti] || (fb2[ti] = [])).push({ e: r[f], s: stc, n: !!inv });
        uf2(ti);
      } else if (o === OP.BIND_TOGGLE) {
        const [f, l] = rv(d, i);
        i += l;
        const t = d[i++],
          ti = d[i++];
        r[f].addEventListener(V[t] || "click", (e) => {
          e.preventDefault();
          fl2[ti] = !fl2[ti];
          uf2(ti);
        });
      } else if (o === OP.INIT_SELECTOR) {
        sl2[d[i]] = d[i + 1];
        i += 2;
      } else if (o === OP.BIND_SELECTOR) {
        const [f, l] = rv(d, i);
        i += l;
        const si = d[i++],
          mv = d[i++];
        const [stc, sl] = rv(d, i);
        i += sl;
        (sb2[si] || (sb2[si] = [])).push({ e: r[f], v: mv, s: stc });
        us2(si);
      } else if (o === OP.BIND_SELECT) {
        const [f, l] = rv(d, i);
        i += l;
        const t = d[i++],
          si = d[i++],
          sv = d[i++];
        r[f].addEventListener(V[t] || "click", (e) => {
          e.preventDefault();
          sl2[si] = sv;
          us2(si);
        });
      } else if (o === OP.BIND_TIMED_TOGGLE) {
        const [f, l] = rv(d, i);
        i += l;
        const t = d[i++],
          ti = d[i++],
          ms = (d[i++] << 8) | d[i++];
        let tm: ReturnType<typeof setTimeout>;
        r[f].addEventListener(V[t] || "click", (e) => {
          e.preventDefault();
          clearTimeout(tm);
          fl2[ti] = true;
          uf2(ti);
          tm = setTimeout(() => {
            fl2[ti] = false;
            uf2(ti);
          }, ms);
        });
      } else if (o === OP.AUTO_TOGGLE) {
        const ti = d[i++],
          ms = (d[i++] << 8) | d[i++];
        setTimeout(() => {
          fl2[ti] = !fl2[ti];
          uf2(ti);
        }, ms);
      } else if (o === OP.BIND_SENTINEL) {
        // One-shot visibility sentinel: fires the handler (with its params)
        // when the element nears the viewport, then disconnects. The server's
        // next render re-keys the binding (params change), so the morph swaps
        // in a fresh node with a live observer — one request in flight is
        // structural. rootMargin preloads a viewport ahead of the fold.
        const [f, fl] = rv(d, i);
        i += fl;
        const [h, hl] = rv(d, i);
        i += hl;
        const pl = d[i++],
          prm = d.slice(i, i + pl);
        i += pl;
        r[f].__hk = "v" + h + "_" + prm.join(",");
        const sel = r[f];
        const ob = new IntersectionObserver(
          (es) => {
            for (const en of es)
              if (en.isIntersecting) {
                ob.disconnect();
                sep(
                  h,
                  OP.EV_VISIBLE,
                  f,
                  prm,
                  { type: "visible", preventDefault() {} } as unknown as Event,
                  sel,
                );
                break;
              }
          },
          { rootMargin: "100%" },
        );
        ob.observe(sel);
      } else if (o === OP.BIND_RESIZE) {
        // Pointer-drag resize handle: dragging resizes the PREVIOUS element
        // sibling's width (client-side only; min 8rem). Adjacency pairing
        // keeps the wire format to a single ref.
        const [f, fl] = rv(d, i);
        i += fl;
        const h = r[f];
        h.__hk = "z";
        h.addEventListener("pointerdown", (e) => {
          const tgt = h.previousElementSibling as HTMLElement | null;
          if (!tgt) return;
          e.preventDefault();
          const sx = (e as PointerEvent).clientX,
            sw = tgt.getBoundingClientRect().width;
          const mv = (ev: PointerEvent) => {
            tgt.style.width = Math.max(128, sw + ev.clientX - sx) + "px";
            tgt.style.flex = "0 0 auto";
          };
          const up = () => {
            removeEventListener("pointermove", mv as EventListener);
            removeEventListener("pointerup", up);
          };
          addEventListener("pointermove", mv as EventListener);
          addEventListener("pointerup", up);
        });
      } else if (o === OP.BATCH_END) {
        fm();
        if (ae) {
          // id lookup first; a generation-re-keyed editor field (undo/redo/
          // reload swap the node AND its id) is found again by data-echo.
          const ne = ((ai && document.getElementById(ai)) ||
            (ae.isConnected
              ? ae
              : ae.getAttribute && ae.getAttribute("data-echo")
                ? document.querySelector("[data-echo]")
                : null)) as RwEl | null;
          if (ne) {
            if (
              av !== null &&
              (ne.tagName === "INPUT" || ne.tagName === "TEXTAREA") &&
              (ne as any).value !== av
            )
              (ne as any).value = av;
            if (ne !== document.activeElement) ne.focus();
            try {
              (ne as any).setSelectionRange(ap, aq);
            } catch (_) {}
          }
        }
        return;
      } else {
        console.error(
          "Unknown opcode 0x" +
            o.toString(16) +
            " at pos " +
            _p +
            " after " +
            _oc +
            " ops, r.len=" +
            r.length,
        );
      }
    }
  } catch (e: any) {
    console.error(
      "PARSE ERROR at pos=" +
        i +
        " op#" +
        _oc +
        " opcode=0x" +
        (d[i - 1] || 0).toString(16) +
        " r.len=" +
        r.length +
        ": " +
        e.message,
    );
    console.error(
      "Context:",
      Array.from(d.slice(Math.max(0, i - 10), i + 10))
        .map((b) => "0x" + b.toString(16).padStart(2, "0"))
        .join(" "),
    );
    try {
      st.w!.close();
    } catch (_) {}
  }
}
