// Scroll to a #hash target; if the element isn't rendered yet (server still
// streaming the page), watch mutations until it appears, 2s safety cutoff.

export function sh(h: string): void {
  if (!h) return;
  const id = h.slice(1);
  if (!id) return;
  const ts = () => {
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: "smooth" });
      return true;
    }
    return false;
  };
  if (!ts()) {
    const ob = new MutationObserver(() => {
      if (ts()) ob.disconnect();
    });
    ob.observe(document.body, { childList: true, subtree: true });
    setTimeout(() => ob.disconnect(), 2000);
  }
}
