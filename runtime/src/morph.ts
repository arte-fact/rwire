// DOM morphing (node reuse on update). me/mk reconcile a live subtree toward a
// freshly-built shadow subtree, reusing nodes by id (and id-less nodes
// positionally) so focus/caret/scroll/uncontrolled input state survive
// updates. A bound node is reused only when its binding key (__hk) is
// unchanged — its existing listener stays valid via stable handler ids; when
// the binding changed, the freshly-built node (with its new listener) is
// swapped in instead, so listeners are never stale and never leak.

import { st, type RwEl } from "./state.ts";

/** Morph one node: text value, attributes, then children (unless it's a
 * nested synced region — its own update owns it). */
export function me(a: Node, b: Node): void {
  if (a.nodeType === 3) {
    if (a.nodeValue !== b.nodeValue) a.nodeValue = b.nodeValue;
    return;
  }
  if (a.nodeType !== 1) return;
  const ae = a as RwEl,
    be = b as RwEl;
  const ba = be.attributes;
  for (let k = 0; k < ba.length; k++) {
    const n = ba[k].name;
    if (ae.getAttribute(n) !== ba[k].value) ae.setAttribute(n, ba[k].value);
  }
  const aa = ae.attributes;
  for (let k = aa.length - 1; k >= 0; k--) {
    const n = aa[k].name;
    if (!be.hasAttribute(n)) ae.removeAttribute(n);
  }
  ae.__hk = be.__hk;
  ae.__k = be.__k;
  ae.__b = be.__b;
  // Nested region: its own update owns it (guard non-string id so the morph
  // never throws and freezes the stream).
  if (typeof ae.id === "string" && ae.id.indexOf("__synced_") === 0) return;
  mk(ae, be);
}

/** Reconcile a's children toward b's: match by id, else positionally for
 * id-less same-type nodes; a changed __hk disqualifies reuse. */
export function mk(a: Element, b: Element): void {
  const byId: Record<string, Element> = {};
  const byKey: Record<number, Element> = {};
  for (let c = a.firstChild; c; c = c.nextSibling)
    if (c.nodeType === 1) {
      if ((c as Element).id) byId[(c as Element).id] = c as Element;
      else if ((c as RwEl).__k !== undefined) byKey[(c as RwEl).__k!] = c as Element;
    }
  let cur = a.firstChild;
  for (let bc = b.firstChild; bc; ) {
    const nb = bc.nextSibling;
    let m: Node | null = null;
    if (bc.nodeType === 1 && (bc as Element).id) {
      if (byId[(bc as Element).id]) m = byId[(bc as Element).id];
    } else if (bc.nodeType === 1 && (bc as RwEl).__k !== undefined) {
      // Keyed: match by sibling-local identity, so reorders move nodes
      // (with their input/scroll/focus state) instead of morphing across.
      if (byKey[(bc as RwEl).__k!]) m = byKey[(bc as RwEl).__k!];
    } else if (
      cur &&
      !(cur.nodeType === 1 &&
        ((cur as Element).id || (cur as RwEl).__k !== undefined)) &&
      cur.nodeType === bc.nodeType &&
      (cur.nodeType !== 1 ||
        (cur as Element).tagName === (bc as Element).tagName)
    )
      m = cur;
    if (
      m &&
      m.nodeType === 1 &&
      ((m as Element).tagName !== (bc as Element).tagName ||
        ((m as RwEl).__hk || "") !== ((bc as RwEl).__hk || ""))
    )
      m = null;
    if (m) {
      if (m !== cur) a.insertBefore(m, cur);
      else cur = cur.nextSibling;
      me(m, bc);
    } else a.insertBefore(bc, cur);
    bc = nb;
  }
  while (cur) {
    const n = cur.nextSibling;
    a.removeChild(cur);
    cur = n;
  }
}

/** Flush the morph staged by CLEAR_CHILDREN (if any). */
export function fm(): void {
  if (st.pm) {
    mk(st.pm.live, st.pm.shadow);
    st.pm = null;
  }
}
