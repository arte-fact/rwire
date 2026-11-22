// Audio DSP shared by the audio extension and its worklet: no DOM, no
// Web Audio, so it runs (and is tested) anywhere.

/** The microphone's rate on the wire. */
export const RATE = 16000;
/** Samples in a 20 ms frame at 16 kHz. */
export const FRAME = 320;

/** A frame ready for the wire. */
export interface Frame {
  seq: number;
  /** Capture time of its first sample, in seconds on the audio clock. */
  t: number;
  pcm: Int16Array;
}

/** A float sample (-1..1) as PCM16, clamped. */
export const toI16 = (x: number): number => {
  const v = Math.round(x * 32767);
  return v > 32767 ? 32767 : v < -32768 ? -32768 : v;
};

/**
 * The capture side: float blocks at the context's rate in, 20 ms frames of
 * 16 kHz PCM16 out. Each output sample is the mean of the input samples it
 * covers (area averaging: a box low-pass for free, so 48 kHz speech does
 * not fold back above 8 kHz); a context slower than 16 kHz is linearly
 * interpolated instead.
 */
export class Capture {
  /** Input samples per output sample. */
  readonly ratio: number;
  private seq = 0;
  /** The input position (in input samples, absolute) of the next output sample's end. */
  private edge: number;
  private acc = 0;
  private n = 0;
  private prev = 0;
  /** Upsampling: the input position of the next output sample. */
  private next = 0;
  private out = new Int16Array(FRAME);
  private fill = 0;
  private t0 = 0;
  /** Absolute input index of the next sample to arrive. */
  private pos = 0;
  private started = false;

  readonly inRate: number;

  constructor(inRate: number) {
    this.inRate = inRate;
    this.ratio = inRate / RATE;
    this.edge = this.ratio;
  }

  /** Feed a block whose first sample is absolute input frame `frame0`; get the frames completed. */
  push(block: Float32Array, frame0: number): Frame[] {
    const done: Frame[] = [];
    if (!this.started) {
      // Align the absolute input position to the clock on the first block.
      this.started = true;
      this.pos = frame0;
      this.edge = frame0 + this.ratio;
      this.next = frame0;
    }
    for (let i = 0; i < block.length; i++) {
      const x = block[i];
      const at = this.pos + i;
      if (this.ratio >= 1) {
        this.acc += x;
        this.n++;
        if (at + 1 >= this.edge) {
          this.emit(this.acc / this.n, at + 1 - this.n, done);
          this.acc = 0;
          this.n = 0;
          this.edge += this.ratio;
        }
      } else {
        // Upsampling: every output position in (at - 1, at], interpolated.
        while (this.next <= at) {
          this.emit(this.prev + (x - this.prev) * (this.next - (at - 1)), this.next, done);
          this.next += this.ratio;
        }
        this.prev = x;
      }
    }
    this.pos += block.length;
    return done;
  }

  private emit(v: number, at: number, done: Frame[]): void {
    if (this.fill === 0) this.t0 = at / this.inRate;
    this.out[this.fill++] = toI16(v);
    if (this.fill === FRAME) {
      done.push({ seq: this.seq++, t: this.t0, pcm: this.out });
      this.out = new Int16Array(FRAME);
      this.fill = 0;
    }
  }
}

/** The wire message of a frame: 0x01, seq u32, t f64, 320 × i16, little-endian (653 bytes). */
export function encodeFrame(f: Frame): ArrayBuffer {
  const buf = new ArrayBuffer(1 + 4 + 8 + FRAME * 2);
  const v = new DataView(buf);
  v.setUint8(0, 0x01);
  v.setUint32(1, f.seq >>> 0, true);
  v.setFloat64(5, f.t, true);
  for (let i = 0; i < FRAME; i++) v.setInt16(13 + i * 2, f.pcm[i], true);
  return buf;
}

/** The RMS level of a frame, 0..1 (computed in the browser, never sent). */
export function level(pcm: Int16Array): number {
  let s = 0;
  for (let i = 0; i < pcm.length; i++) s += pcm[i] * pcm[i];
  return pcm.length ? Math.sqrt(s / pcm.length) / 32768 : 0;
}
