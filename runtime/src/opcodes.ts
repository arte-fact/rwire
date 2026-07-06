// Wire opcode byte values. MUST match libs/rwire/src/protocol/opcodes.rs —
// this file becomes generated code in RT3 (regen script emits it from the Rust
// constants); until then, hand-edits go in the Rust file first.
export const OP = {
  S: 0xf0, // SYMBOLS
  SE: 0xf1, // SYMBOLS_EXTEND
  WT: 0xf2, // WORD_TABLE
  G: 0x01, // GET_BY_ID
  C: 0x02, // CREATE
  CS: 0x03, // CREATE_SYNCED
  GS: 0x05, // GET_SYNCED
  L: 0x10, // SET_CLASS
  T: 0x11, // SET_TEXT
  TW: 0x13, // SET_TEXT_WORDS
  D: 0x14, // SET_DATA
  TI: 0x15, // SET_TEXT_INT
  A: 0x12, // SET_ATTR
  P: 0x20, // APPEND
  CC: 0x25, // CLEAR_CHILDREN (morph staging)
  AE: 0x26, // SET_ATTR_ENUM
  AB: 0x27, // SET_ATTR_BOOL
  AK: 0x28, // SET_ATTR_KEY_SYM
  B: 0x30, // BIND_LOCAL
  R: 0x31, // BIND_REMOTE
  DB: 0x33, // BIND_DEBOUNCED
  RP: 0x34, // BIND_REMOTE_PARAM
  IL: 0x40, // inline-local handler (WASM builds; stubbed here)
  DH: 0x42, // handler def (WASM builds; stubbed here)
  IT: 0x47, // INIT_TARGET
  BT: 0x48, // BIND_TARGET
  TG: 0x49, // BIND_TOGGLE
  IS: 0x4a, // INIT_SELECTOR
  BS: 0x4b, // BIND_SELECTOR
  SS2: 0x4c, // BIND_SELECT
  TT: 0x4d, // BIND_TIMED_TOGGLE
  AT2: 0x4e, // AUTO_TOGGLE
  RU: 0x70, // ROUTE_PUSH
  RR: 0x71, // ROUTE_REPLACE
  RUI: 0x72, // ROUTE_PUSH_INLINE
  RRI: 0x73, // ROUTE_REPLACE_INLINE
  SS: 0x81, // STYLE_SET
  SU: 0x82, // STYLE_UTIL
  SP: 0x83, // STYLE_PROP
  SM: 0x84, // STYLE_MULTI
  SC: 0x85, // STYLE_COMPOSITE
  CT: 0x86, // COMPOSITE_TABLE
  SD: 0x87, // STYLE_DEF (lazy CSS)
  MD: 0x88, // MAP_DEF (lazy name maps)
  PD: 0x89, // STYLE_PSEUDO
  BP: 0x8a, // STYLE_BREAKPOINT
  E: 0xff, // BATCH_END
} as const;
