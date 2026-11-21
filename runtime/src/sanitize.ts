// Attribute sanitizer (audit finding L3): symbol-table attr values are
// server-authored, but defense in depth still refuses event-handler attrs and
// URLs that run code in URL-bearing attributes: `javascript:`, `vbscript:`,
// and `data:` other than an image in `src` (an SVG one can carry script),
// after dropping controls and spaces (`java\tscript:`) and lowercasing. Same
// rule as the server's pre-rendered HTML (`rwire::url_safety`).

/** setAttribute, refusing `on*` names and executable URLs in URL attrs. The
 * name keeps its case: SVG attributes (`viewBox`) are case-sensitive. */
export function sa(e: Element, n: string, v: string): void {
  const l = ("" + n).toLowerCase();
  if (l.slice(0, 2) === "on") return;
  const u = ("" + v).replace(/[\u0000-\u0020]+/g, "").toLowerCase();
  if (
    /^(href|src|xlink:href|formaction|action)$/.test(l) &&
    /^((java|vb)script|data):/.test(u) &&
    !(l === "src" && /^data:image[\/;,](?!svg)/.test(u))
  )
    return;
  e.setAttribute(n, v);
}
