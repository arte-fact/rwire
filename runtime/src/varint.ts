// Varint codec — MUST mirror libs/rwire/src/protocol/varint.rs exactly.
// 1 byte:  0x00-0x7F            → 0..127
// 2 bytes: 0x80|hi, lo          → 128..16511 (value - 128 in 14 bits)
// 3 bytes: 0xC0|hi, mid, lo     → 16512..    (value - 16512 in 22 bits)

/** Read a varint at `i`; returns [value, bytesConsumed]. */
export function rv(d: Uint8Array, i: number): [number, number] {
  const b = d[i];
  if (b < 0x80) return [b, 1];
  if (b < 0xc0) return [0x80 + ((b & 0x3f) << 8) + d[i + 1], 2];
  return [0x4080 + ((b & 0x3f) << 16) + (d[i + 1] << 8) + d[i + 2], 3];
}

/** Append the varint encoding of `v` to `a`. */
export function wv(a: number[], v: number): void {
  if (v < 128) a.push(v);
  else if (v < 16512) {
    v -= 128;
    a.push(128 | (v >> 8), v & 255);
  } else {
    v -= 16512;
    a.push(192 | (v >> 16), (v >> 8) & 255, v & 255);
  }
}
