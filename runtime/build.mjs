// Build the runtime artifact: bundle src/index.ts into one minified IIFE.
// - Identifier mangling only. Property names are NEVER mangled: `__hk`, `__t`,
//   and the DOM API surface are load-bearing.
// - `BASE` stays a free (external) global — the capsule injects it.
// - dist/runtime.js (readable) is emitted beside dist/runtime.min.js for
//   debugging; only the .min.js is embedded by the rwire crate (RT2).
import { build } from "esbuild";
import { readFileSync, writeFileSync } from "node:fs";
import { gzipSync } from "node:zlib";

const shared = {
  entryPoints: ["src/index.ts"],
  bundle: true,
  format: "iife",
  target: "es2020",
  charset: "utf8",
  legalComments: "none",
};

await build({ ...shared, outfile: "dist/runtime.js", minify: false });
await build({ ...shared, outfile: "dist/runtime.min.js", minify: true });

// Strip the trailing newline esbuild appends so the embedded artifact is a
// single clean line, then report sizes (the capsule-size budget tracks these).
const min = readFileSync("dist/runtime.min.js", "utf8").trimEnd();
writeFileSync("dist/runtime.min.js", min);
const gz = gzipSync(Buffer.from(min)).length;
console.log(`runtime.min.js: ${min.length} bytes (${gz} gzipped)`);
