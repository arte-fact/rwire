// The audio extension's worklet module, loaded by `audioWorklet.addModule`
// from `/_rw/ext/audio-worklet.js`. It runs on the audio thread:
// - `rw-capture`: the microphone's blocks in, 20 ms frames of 16 kHz PCM16
//   out to the main thread (with their audio-clock time).

import { Capture } from "./audio-dsp.ts";

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
