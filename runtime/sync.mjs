// The ONLY write path for the embedded runtime artifact: build from source,
// then copy into the rwire crate's assets. CI rebuilds and fails on git diff,
// so a hand-edited or stale libs/rwire/assets/runtime.min.js cannot land.
import "./build.mjs";
import { copyFileSync } from "node:fs";

const dest = new URL("../libs/rwire/assets/runtime.min.js", import.meta.url);
copyFileSync(new URL("dist/runtime.min.js", import.meta.url), dest);
console.log(`synced → ${dest.pathname}`);
