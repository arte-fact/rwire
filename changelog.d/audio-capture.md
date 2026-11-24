### Added: microphone capture in `rwire-audio` (RW1, RW3)

- **`Audio::capture(bool)`**: each render starts or stops the microphone.
  - When on, the browser asks for it with `echoCancellation`, `noiseSuppression` and `autoGainControl`.
  - The `rw-capture` AudioWorklet resamples to 16 kHz by area averaging, a box low-pass that keeps speech from folding back. It sends 20 ms frames of 320 PCM16 samples, each with a sequence number and its time on the page's audio clock.
  - `Frame.seq` restarts at 0 at each capture start.
  - When off, every track is stopped, so the browser's microphone indicator goes off.
- **`Audio::on_event(handler)` and `parse_event(ctx)`**: events reach the app through the usual handler path.
  - `AudioEvent::Capture(Granted | Denied | Unavailable | Insecure)`.
  - `AudioEvent::Output(Ready | Blocked | Interrupted)`. The extension resumes the audio context on any user gesture in the page, inside the gesture, so playback survives the autoplay policy.
- **Socket lifetime**: the `audio` extension opens the audio socket only while the page has an `Audio` element.
  - It reconnects by itself with the ticket handed over by the previous connection.
  - It does not retry a refused ticket (4403).
- **Level**: the level of the last frame is set as `--rw-level` on the element, for a meter, and is never sent.
- **Serving**: `serve_audio` also serves `/_rw/ext/audio.js` and `/_rw/ext/audio-worklet.js`.
