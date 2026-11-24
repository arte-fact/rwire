### Added: gapless playback in `rwire-audio` (RW2), and an example

- **The `rw-play` worklet** plays what `link.output` queues, in call order, with no gaps:
  - PCM at any rate (`pcm(samples, rate)`), resampled in the browser and continuous across chunks;
  - preloaded sounds (`Audio::sounds([(name, url)])`, `output.sound(name)`), never mixed over the PCM;
  - marks and the end of the utterance (`output.end()`).
- **New `AudioEvent`s**, each timed on the page's audio clock at the moment the sample is heard (output latency added):
  - `PlaybackStarted`, `MarkReached { id, at }`, `PlaybackEnded` (only after `end()`), `Underrun` (the queue ran dry before `end()`);
  - `Stopped { last_mark, samples_since }`, after `output.stop()`, which drops the queue at the next render quantum.
- **Report events**: the extension reports with `change` events, not `input`, so lines written in one burst are not merged by the input debounce.
- **`examples/audio`**: speak, then hear it back after a ding; a tone with marks; stop.
- **`runtime/e2e/audio-chrome.mjs`**: drives headless Chrome with its fake microphone against the example and checks:
  - capture, playback events and their times;
  - stop latency, 90 ms measured, target 300 ms;
  - echo: the loudest microphone frame while a tone plays.
