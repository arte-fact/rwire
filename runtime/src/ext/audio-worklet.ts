// The audio extension's worklet module, loaded by `audioWorklet.addModule`
// from `/_rw/ext/audio-worklet.js`. It runs on the audio thread:
// - `rw-capture`: the microphone's blocks in, 20 ms frames of 16 kHz PCM16
//   out to the main thread (with their audio-clock time);
// - `rw-play`: the playback queue (see `Player`), fed by the main thread,
//   and its notes (started, mark, underrun, ended, stopped) back to it, by
//   render frame.

import { Capture } from "./audio-dsp.ts";
import { Player, type Note } from "./audio-play.ts";

declare const sampleRate: number;
declare const currentFrame: number;
declare function registerProcessor(name: string, ctor: unknown): void;
declare class AudioWorkletProcessor {
  readonly port: MessagePort;
}

class RwCapture extends AudioWorkletProcessor {
  private c = new Capture(sampleRate);

  process(inputs: Float32Array[][]): boolean {
    const ch = inputs[0] && inputs[0][0];
    if (ch) for (const f of this.c.push(ch, currentFrame)) this.port.postMessage(f, [f.pcm.buffer]);
    return true;
  }
}

registerProcessor("rw-capture", RwCapture);

class RwPlay extends AudioWorkletProcessor {
  private p = new Player();

  constructor() {
    super();
    this.port.onmessage = (m: MessageEvent) => {
      const d = m.data;
      if (d.k === "stop") this.notes(this.p.stop(currentFrame));
      else this.p.push(d);
    };
  }

  private notes(ns: Note[]): void {
    for (const n of ns) this.port.postMessage(n);
  }

  process(_inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    const out = outputs[0] && outputs[0][0];
    if (out) this.notes(this.p.render(out, currentFrame));
    return true;
  }
}

registerProcessor("rw-play", RwPlay);
