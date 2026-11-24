# rwire — Microphone and playback (`rwire-audio`)

The official `audio` extension: microphone capture and gapless playback over a dedicated
WebSocket per page. Never write free JavaScript for audio. Example: `examples/audio`;
browser check: `runtime/e2e/audio-chrome.mjs`. This page is kept from the in-repo skill and
re-checked against `libs/rwire-audio/src`.

## What the crate provides

- **Server** — `.serve_audio(|mut link: AudioLink| async move { … })` on the server builder.
  The page's audio rides its own socket, `/_rw/ext/audio/ws`, authenticated by a single-use
  ticket bound to the session, identity and connection; each connection hands the extension
  the ticket of the next one, so a lost socket reconnects by itself with a new `AudioLink`
  (same `session()` / `name()`, a new `connection_id()`). The task given to `serve_audio`
  runs once per connection and lives as long as the link: while it lives it is never
  stopped; when the socket is gone `input.recv()` returns `None` and `output.closed()`
  resolves, and a task still running 5 s later is aborted.
- **Input** — `link.input.recv()` yields 20 ms `Frame`s: `seq` (restarts at 0 at each capture
  start), `timestamp` (seconds on the page's audio clock) and 320 PCM16 samples at 16 kHz.
  The queue is bounded (50 frames, one second) and drops the oldest, so the gap shows in
  `seq`: read the frames and run the VAD in the task, and feed transcription, synthesis and
  hub calls from other tasks through channels.
- **Output** — `link.output` offers `pcm(&samples, rate)`, `mark(id)`, `sound(name)` and
  `end()`, each returning `Result<(), OutputError>` (`QueueFull` past a ~60 s-at-24 kHz
  budget, `Closed`, `RateRefused` outside 8000..=192000 Hz; nothing is queued on error), and
  `stop()`, which returns nothing and is never refused. Everything is queued in call order
  and played gaplessly; nothing is ever mixed over anything. PCM may come at any rate (most
  TTS output is 24 kHz); the browser resamples.
- **Link accessors** — `link.session()`, `link.name()`, `link.connection_id()`,
  `link.identity()`, and `link.server()` for `update_shared::<T>(…)`, which changes a shared
  state and re-renders the pages that show it (how a server-side VAD drives the microphone
  button).
- **Page** — `el(El::Div).audio(Audio::new("voice").capture(me.listening).sounds([("ding",
  "/sounds/ding.wav")]).on_event(h()))`. The render drives the microphone (echo cancellation,
  noise suppression and automatic gain requested); `capture(false)` stops every track and
  the browser's indicator goes off. One stream per page. A page with an audio element holds
  two connections (page socket + audio socket): size `max_connections` for it.
- **Handler** — `parse_event(ctx)` returns an `AudioEvent`: `Capture(Granted | Denied |
  Unavailable | Insecure)`, `Output(Ready | Blocked | Interrupted)`, `PlaybackStarted { at }`,
  `MarkReached { id, at }`, `Underrun { at }`, `PlaybackEnded { at }`, `Stopped { at,
  last_mark, samples_since }`. Every `at` is in seconds on the browser's audio clock — the
  clock of the frames' `timestamp` — at the moment the sample is heard (output latency
  included).

## Traps

- `PlaybackEnded` comes only after `output.end()`; a queue that runs dry before it raises
  `Underrun` and the utterance goes on when more PCM arrives.
- `output.stop()` drops the queue and reports `Stopped { last_mark, samples_since }`: where
  the reading stopped ("partly read" comes from `last_mark`).
- The microphone needs a secure context (https, or `localhost` / `127.0.0.1`), otherwise
  `Capture(Insecure)`.
- A context created before any user gesture is `Output(Blocked)` until one; the extension
  resumes it inside any gesture of the page — including the tap that turns the microphone
  on — so the output survives the autoplay policy.
- Playback and capture events are the client's claims, not proofs: use them to drive the UI
  and the conversation, never for authorization, billing or accounting.
- The microphone level is computed in the browser only (`--rw-level` on the element, for a
  meter); it is never sent.
- The wire format is public (`rwire_audio::wire`), so a non-browser client can speak the
  same socket in tests: frames `0x01`, and from the server ticket `0x80`, PCM `0x81`, mark
  `0x82`, sound `0x83`, stop `0x84`, end `0x85`.

## Checking it in a real browser

`runtime/e2e/audio-chrome.mjs` drives headless Chrome against a live `cargo run -p audio`:
capture granted and frames flowing, a tone with marks reported on the audio clock,
`Stopped` after a stop, and the echo check — the loudest microphone frame while a tone
plays stays under 0.01. Chrome runs with `--use-fake-device-for-media-stream` and
`--use-file-for-fake-audio-capture=<silence.wav>`; the fake microphone has no acoustic
path from the speakers, so this proves that no output leaks into the captured frames
inside the pipeline. The browser's echo cancellation itself is checked by hand, with
speakers and a microphone in a room: open the page, turn the microphone on, play a tone,
and read the loudest captured frame.
