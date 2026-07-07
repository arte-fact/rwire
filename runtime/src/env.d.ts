// Globals injected by the capsule <script> BEFORE the runtime bundle.
// `BASE` is the mount path for reverse-proxied apps (empty string at root);
// see base_path_js() in capsule_gen.rs — the only dynamic config the runtime
// reads. Everything else the runtime needs lives inside the bundle.
declare const BASE: string;
declare const RWV: string;
