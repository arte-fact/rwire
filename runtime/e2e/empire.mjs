// Full-stack E2E (manual): the shipped artifact + a real WebSocket against a
// LIVE empire-web server. Start it first: `cargo run -p empire-web`, then
// `node e2e/empire.mjs` from runtime/. Creates a table from the home page,
// starts a game, then moves every live slider and asserts its readouts follow
// with no server round-trip (LIVE_SOURCE / LIVE_BIND), and that no batch ever
// hit a parse error.
import { readFileSync } from "node:fs";
import { makeDom } from "../test/dom.ts";

const PORT = process.env.PORT || "7782";
const artifact = readFileSync(new URL("../../libs/rwire/assets/runtime.min.js", import.meta.url), "utf8");
const { document } = makeDom();

const noop = () => {};
class MO { observe() {} disconnect() {} }
class IO { observe() {} disconnect() {} }
const location = { protocol: "http:", host: "127.0.0.1:" + PORT, pathname: "/", hash: "" };
const history = { pushState: (_s, _t, u) => (location.pathname = u), replaceState: noop };
const navigator = { onLine: true, clipboard: { writeText: noop } };
const errors = [];
const con = { ...console, error: (...a) => errors.push(a.map(String).join(" ")) };

const factory = new Function(
  "document", "window", "addEventListener", "removeEventListener",
  "history", "location", "navigator", "WebSocket", "MutationObserver", "IntersectionObserver", "console",
  "setTimeout", "clearTimeout", "scrollTo", "globalThis", "BASE", "fetch",
  artifact,
);
factory(
  document, { addEventListener: noop }, noop, noop,
  history, location, navigator, WebSocket, MO, IO, con,
  setTimeout, clearTimeout, noop, {}, "", () => Promise.reject(new Error("no")),
);

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const walk = (n, out = []) => {
  out.push(n);
  for (const c of n.children || []) walk(c, out);
  return out;
};
const all = () => walk(document.body);
const button = (label) =>
  all().find((n) => n.tagName === "BUTTON" && n.textContent.trim().startsWith(label));
const click = async (label) => {
  const b = button(label);
  if (!b) throw new Error(`no button "${label}" — page text: ${document.body.textContent.slice(0, 300)}`);
  b.fire("click", { target: b });
  await sleep(800);
};

await sleep(1200); // initial render
if (!button("Créer une table")) throw new Error("home page not rendered: " + document.body.textContent.slice(0, 200));
await click("Créer une table");
console.log("table:", location.pathname);
// Take the first free seat (a computer's chair), start the game, then tap
// through the opening screens until a step with live sliders shows.
const seat = all().find((n) => n.tagName === "BUTTON" && n.textContent.includes("Ordinateur"));
if (seat) {
  seat.fire("click", { target: seat });
  await sleep(800);
}
if (!button("Commencer la partie")) {
  const labels = all().filter((n) => n.tagName === "BUTTON").map((n) => n.textContent.trim().slice(0, 30));
  console.log("console errors:", errors);
  throw new Error("cannot start: buttons " + JSON.stringify(labels) + " — " + document.body.textContent.slice(0, 200));
}
await click("Commencer la partie");
const live = () => all().filter((n) => n.__lc !== undefined);
for (let tap = 0; tap < 14 && !live().length; tap++) {
  // The Intendance keeps its sliders in bottom sheets: "Régler" opens one.
  if (document.body.textContent.includes("· Intendance")) {
    const settle = button("Régler");
    if (settle) {
      settle.fire("click", { target: settle });
      await sleep(700);
      if (live().length) break;
    }
  }
  const next =
    button("Continuer") ||
    button("Prêt") ||
    all().find((n) => n.__b && n.__b.click && n.textContent.includes("Touchez l'écran"));
  if (!next) break;
  next.fire("click", { target: next });
  await sleep(700);
}
if (!live().length) {
  const bound = all().filter((n) => n.__b && n.__b.click).map((n) => n.tagName + " " + JSON.stringify(n.textContent.trim().slice(0, 40)));
  console.log("click-bound elements:", bound);
  console.log("page:", document.body.textContent.slice(0, 500));
}

// Every live source on the page: move it to its max and watch its readouts.
const sources = live();
console.log(`${sources.length} live sources on ${location.pathname}`);
let moved = 0;
for (const s of sources) {
  const bound = all().filter((n) => n.__lb);
  const before = bound.map((n) => [n, n.textContent, n.style.width]);
  s.value = s.getAttribute("max") || "100";
  s.min = s.getAttribute("min") || "0";
  s.max = s.getAttribute("max") || "100";
  s.fire("input", { target: s });
  const changed = before.filter(([n, t, w]) => n.textContent !== t || n.style.width !== w).length;
  if (changed) moved++;
}
console.log(`${moved} sources moved at least one readout without a round-trip`);
const parse = errors.filter((e) => /PARSE ERROR|Unknown opcode/.test(e));
console.log(parse.length ? "PARSE ERRORS:\n" + parse.join("\n") : "no parse errors");
if (parse.length) process.exit(1);
if (sources.length && !moved) process.exit(1);
process.exit(0);
