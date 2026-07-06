// Reconnect/offline overlay: one fixed full-screen layer, created on first
// show, reused after. Inline styles only — the app's CSS may be gone while
// disconnected; theme variables degrade to readable fallbacks.

import { retryConnect } from "./connect.ts";

export function ov(show: boolean, off?: boolean): void {
  let o = document.getElementById("__rwov");
  if (!o) {
    o = document.createElement("div");
    o.id = "__rwov";
    o.style.cssText =
      "position:fixed;inset:0;z-index:2147483647;display:flex;align-items:center;justify-content:center;background:rgba(0,0,0,.45);font-family:inherit";
    o.innerHTML =
      '<div style="background:var(--a,#fff);color:var(--k,#111);border:1px solid var(--c,rgba(128,128,128,.3));border-radius:.5rem;padding:1.1rem 1.4rem;text-align:center;max-width:18rem"><div id="__rwovm" style="font-weight:600;margin-bottom:.7rem"></div><button id="__rwovr" style="font:inherit;cursor:pointer;border:1px solid var(--c,rgba(128,128,128,.4));background:var(--r,rgba(128,128,128,.12));color:inherit;border-radius:.375rem;padding:.35rem .85rem">Retry</button></div>';
    document.body.appendChild(o);
    (o.querySelector("#__rwovr") as HTMLElement).onclick = () => retryConnect();
  }
  (o.querySelector("#__rwovm") as HTMLElement).textContent = off
    ? "You’re offline"
    : "Reconnecting…";
  o.style.display = show ? "flex" : "none";
}
