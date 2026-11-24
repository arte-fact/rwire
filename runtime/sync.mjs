// The ONLY write path for the embedded runtime artifact: build from source,
// then copy into the rwire crate's assets. CI rebuilds and fails on git diff,
// so a hand-edited or stale libs/rwire/assets/runtime.min.js cannot land.
import { EXTS } from "./build.mjs";
import { copyFileSync, mkdirSync } from "node:fs";

const dest = new URL("../libs/rwire/assets/runtime.min.js", import.meta.url);
copyFileSync(new URL("dist/runtime.min.js", import.meta.url), dest);
console.log(`synced → ${dest.pathname}`);
// Where each extension is vendored: vim in the core crate, map in rwire-map,
// push in rwire-push, audio and its worklet in rwire-audio.
const EXT_DEST = {
  vim: "../libs/rwire/assets/ext/vim.min.js",
  map: "../libs/rwire-map/assets/map.min.js",
  push: "../libs/rwire-push/assets/push.min.js",
  audio: "../libs/rwire-audio/assets/audio.min.js",
  "audio-worklet": "../libs/rwire-audio/assets/audio-worklet.min.js",
};
for (const name of EXTS) {
  const extDest = new URL(EXT_DEST[name], import.meta.url);
  mkdirSync(new URL(".", extDest), { recursive: true });
  copyFileSync(new URL(`dist/ext/${name}.min.js`, import.meta.url), extDest);
  console.log(`synced → ${extDest.pathname}`);
}
