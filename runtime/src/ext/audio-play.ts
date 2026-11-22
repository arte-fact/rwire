// Playback shared by the audio extension and its worklet: no DOM, no Web
// Audio, so it runs (and is tested) anywhere.
//
// The main thread resamples what the server queues to the context's rate
// (`Resampler`) and hands it, in order, to the `rw-play` worklet, whose
// `Player` plays it gaplessly and says, by render time, when an utterance
// starts, when a mark's sample is rendered, when the queue runs dry before
// the utterance's end, when the end is reached, and when a stop drops it.

/**
 * Linear interpolation from `inRate` to `outRate`, continuous across
 * chunks: the fractional position and the last sample carry over.
 */
export class Resampler {
  readonly step: number;
  /** Position of the next output sample, in input samples from the start of the next chunk (-1 is the previous chunk's last sample). */
  private t = 0;
  private last = 0;

  constructor(inRate: number, outRate: number) {
    this.step = inRate / outRate;
  }

  push(pcm: Int16Array): Float32Array {
    const n = pcm.length;
    const x = (k: number) => (k < 0 ? this.last : pcm[k]) / 32768;
    const out: number[] = [];
    let t = this.t;
    while (t <= n - 1) {
      const i = Math.floor(t);
      const f = t - i;
      out.push(f === 0 ? x(i) : x(i) + (x(i + 1) - x(i)) * f);
      t += this.step;
    }
    if (n) {
      this.t = t - n;
      this.last = pcm[n - 1];
    }
    return Float32Array.from(out);
  }
}

/** What the worklet's queue holds. */
export type Item =
  | { k: "pcm"; data: Float32Array; /** Source samples per output sample. */ ratio: number }
  | { k: "mark"; id: string }
  | { k: "end" };

/** What the player reports, `at` being the output frame (context clock × rate) where it happens. */
export type Note =
  | { e: "started"; at: number }
  | { e: "mark"; at: number; id: string }
  | { e: "underrun"; at: number }
  | { e: "ended"; at: number }
  | { e: "stopped"; at: number; mark: string; since: number };

/**
 * The gapless player: one queue, played in order, one sample after the
 * other. Silence only when the queue is empty.
 */
export class Player {
  private q: Item[] = [];
  /** Read position in the head item (pcm). */
  private off = 0;
  /** An utterance is open: it started and has not ended or stopped. */
  private open = false;
  /** Waiting for more after running dry. */
  private dry = false;
  private lastMark = "";
  /** Source samples played since the last mark (fractional). */
  private since = 0;

  push(item: Item): void {
    this.q.push(item);
  }

  /** Whether anything is queued. */
  get queued(): number {
    return this.q.length;
  }

  /** Drop everything now; a note if an utterance was open. */
  stop(at: number): Note[] {
    const was = this.open || this.q.length > 0;
    this.q = [];
    this.off = 0;
    this.open = false;
    this.dry = false;
    const note: Note[] = was ? [{ e: "stopped", at, mark: this.lastMark, since: Math.round(this.since) }] : [];
    this.lastMark = "";
    this.since = 0;
    return note;
  }

  /** Fill `out` (a render quantum starting at output frame `frame0`); the notes it produced. */
  render(out: Float32Array, frame0: number): Note[] {
    const notes: Note[] = [];
    let i = 0;
    while (i < out.length) {
      const head = this.q[0];
      if (!head) break;
      if (head.k === "mark") {
        this.q.shift();
        if (!this.open) notes.push({ e: "started", at: frame0 + i });
        this.open = true;
        this.lastMark = head.id;
        this.since = 0;
        notes.push({ e: "mark", at: frame0 + i, id: head.id });
        continue;
      }
      if (head.k === "end") {
        this.q.shift();
        if (this.open) notes.push({ e: "ended", at: frame0 + i });
        this.open = false;
        this.dry = false;
        this.lastMark = "";
        this.since = 0;
        continue;
      }
      if (!this.open) {
        this.open = true;
        notes.push({ e: "started", at: frame0 + i });
      }
      this.dry = false;
      const n = Math.min(out.length - i, head.data.length - this.off);
      out.set(head.data.subarray(this.off, this.off + n), i);
      i += n;
      this.off += n;
      this.since += n * head.ratio;
      if (this.off >= head.data.length) {
        this.q.shift();
        this.off = 0;
      }
    }
    if (i < out.length) {
      out.fill(0, i);
      if (this.open && !this.dry) {
        this.dry = true;
        notes.push({ e: "underrun", at: frame0 + i });
      }
    }
    return notes;
  }
}
