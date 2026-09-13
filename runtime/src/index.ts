// Bundle entry — bootstrap side effects in the original capsule's order:
// manual scroll restoration, first connect, resilience listeners, router glue.

import { x } from "./executor.ts";
import {
  connect,
  reconnectIfDead,
  offlineNotice,
  onVisibilityChange,
} from "./connect.ts";
import { installRouter } from "./router.ts";

if ("scrollRestoration" in history) history.scrollRestoration = "manual";
connect();
addEventListener("online", reconnectIfDead);
addEventListener("offline", offlineNotice);
document.addEventListener("visibilitychange", onVisibilityChange);
installRouter();

// Stable debug/testing handle: the wire harness (and devtools) drive the
// opcode executor through this.
(globalThis as any).__rwx = x;
