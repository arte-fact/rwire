// Attribute sanitizer (audit finding L3): symbol-table attr values are
// server-authored, but defense in depth still refuses event-handler attrs and
// javascript: URLs in URL-bearing attributes.

/** setAttribute, refusing `on*` names and javascript: URLs in URL attrs. The
 * name keeps its case: SVG attributes (`viewBox`) are case-sensitive. */
export function sa(e: Element, n: string, v: string): void {
  const l = ("" + n).toLowerCase();
  if (l.slice(0, 2) === "on") return;
  if (
    /^(href|src|xlink:href|formaction|action)$/.test(l) &&
    /^javascript:/.test(
      ("" + v).replace(/[\u0000-\u0020]+/g, "").toLowerCase(),
    )
  )
    return;
  e.setAttribute(n, v);
}
