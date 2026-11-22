//! Microphone capture and gapless playback for rwire (the official
//! `audio` runtime extension: no free JavaScript in the application).
//!
//! # Transport
//!
//! The audio of a page does not ride the page's socket: it has a socket of
//! its own, `/_rw/ext/audio/ws` ([`AUDIO_WS_PATH`]), registered by
//! [`ServeAudio::serve_audio`] and opened by the extension only while the
//! page renders an [`Audio`] element (one per page). It is authenticated by
//! a single-use ticket the render issues, bound to the page's session,
//! identity and connection (`rwire::ext_socket`); each connection hands the
//! extension the ticket of the next one, so a lost socket reconnects by
//! itself, with no render, and gives a new [`AudioLink`] with the same
//! [`AudioLink::session`], [`AudioLink::name`] and
//! [`AudioLink::connection_id`] (two tabs of a session have two ids).
//!
//! # Usage
//!
//! ```ignore
//! use rwire_audio::{parse_event, Audio, AudioEvent, AudioExt, AudioLink, ServeAudio};
//!
//! #[renderer]
//! fn mic(me: &Me) -> ElementBuilder {
//!     el(El::Div).audio(
//!         Audio::new("voice")
//!             .capture(me.listening) // each render starts or stops the microphone
//!             .sounds([("ding", "/sounds/ding.wav")])
//!             .on_event(on_audio()),
//!     )
//! }
//!
//! #[handler]
//! fn on_audio(me: &mut Me, ctx: &EventContext) {
//!     match parse_event(ctx) {
//!         Some(AudioEvent::Capture(CaptureState::Denied)) => me.alert = Some("microphone refused"),
//!         Some(AudioEvent::MarkReached { id, at }) => me.read_up_to = id,
//!         Some(AudioEvent::Stopped { last_mark, samples_since, .. }) => { /* partly read */ }
//!         _ => {}
//!     }
//! }
//!
//! Server::bind(addr)?
//!     .root(root)
//!     .serve_audio(|mut link: AudioLink| async move {
//!         while let Some(frame) = link.input.recv().await {
//!             // frame.seq, frame.timestamp, frame.samples: [i16; 320] at 16 kHz
//!             if let Some(speech) = vad.push(&frame) {
//!                 let pcm = tts(&answer(speech)).await; // 24 kHz
//!                 link.output.sound("ding");
//!                 link.output.pcm(&pcm, 24_000);
//!                 link.output.mark("s1");
//!                 link.output.end();
//!             }
//!             // barge-in: link.output.stop();
//!             link.server().update_shared::<Hall>(|h| h.heard += 1); // re-renders
//!         }
//!     })
//!     .run()
//!     .await
//! ```
//!
//! `examples/audio` is a complete app (speak, then hear it back; a tone
//! with marks; stop).
//!
//! # Rules
//!
//! - **Frames.** [`Frame::seq`] restarts at 0 at each capture start. If the
//!   application reads slower than the microphone, the oldest frames are
//!   dropped from a bounded queue ([`INPUT_CAPACITY`], one second): the gap
//!   shows in `seq`.
//! - **One clock.** [`Frame::timestamp`] and every `at` of the playback
//!   events are seconds on the page's `AudioContext` clock. A playback event
//!   is reported when its sample is heard (output latency added).
//! - **The queue.** PCM (any rate, resampled by the browser), marks, sounds
//!   and the end are played in call order, gaplessly, nothing mixed over
//!   anything. `PlaybackEnded` comes only after [`AudioOutput::end`]; a queue
//!   that runs dry before it is an `Underrun`. [`AudioOutput::stop`] drops
//!   the queue at the next render quantum and reports `Stopped` with the
//!   last mark and the samples heard since.
//! - **Autoplay.** The browser keeps an audio context made without a user
//!   gesture suspended (`Output(Blocked)`); the extension resumes it inside
//!   any gesture of the page, such as the tap that turns the microphone on.
//! - **Level.** The level of the microphone is computed in the browser only
//!   (`--rw-level` on the element, for a meter); it is never sent.
//!
//! # Checking it in a real browser
//!
//! `runtime/e2e/audio-chrome.mjs` drives headless Chrome against
//! `cargo run -p audio`: capture, playback events and their times, stop
//! latency, and an echo check (the loudest microphone frame while a tone
//! plays). It starts Chrome with Chromium's fake microphone:
//!
//! ```text
//! google-chrome --headless=new --use-fake-ui-for-media-stream \
//!   --use-fake-device-for-media-stream \
//!   --use-file-for-fake-audio-capture=/path/to/input.wav
//! ```
//!
//! The fake microphone plays the WAV file in a loop (silence for the echo
//! check, recorded speech to try a voice pipeline). It has no acoustic path
//! from the speakers, so this proves that no output leaks into the captured
//! frames inside the pipeline; the quality of the browser's echo
//! cancellation itself is checked by hand, with speakers and a microphone in
//! a room: open the page, turn the microphone on, play a tone, and read
//! `playmax`.

mod event;
mod link;
pub mod wire;

use std::borrow::Cow;
use std::future::Future;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use rwire::ext_socket::tokio_tungstenite::tungstenite::Message;
use rwire::{El, ElementBuilder, Ev, ExtSocket, HandlerSpec, ServerWithRoot};
use tokio::sync::mpsc;

pub use event::{parse_event, AudioEvent, CaptureState, OutputState};
pub use link::{AudioInput, AudioLink, AudioOutput, INPUT_CAPACITY};
pub use wire::{Frame, FRAME_SAMPLES, RATE};

/// The audio socket's path.
pub const AUDIO_WS_PATH: &str = "/_rw/ext/audio/ws";

/// Where the runtime loader asks for the extension once a batch hints `ext("audio")`.
pub const AUDIO_JS_PATH: &str = "/_rw/ext/audio.js";
/// The extension, minified: built from `runtime/src/ext/audio.ts` by `npm run sync`.
pub const AUDIO_JS: &str = include_str!("../assets/audio.min.js");
/// Where the extension loads its worklet module from.
pub const WORKLET_JS_PATH: &str = "/_rw/ext/audio-worklet.js";
/// The worklet module, minified: built from `runtime/src/ext/audio-worklet.ts`.
pub const WORKLET_JS: &str = include_str!("../assets/audio-worklet.min.js");

/// A page's audio stream, rendered with [`AudioExt::audio`]. One per page.
pub struct Audio {
    name: String,
    capture: bool,
    on_event: Option<HandlerSpec>,
    sounds: Vec<(String, String)>,
}

impl Audio {
    /// A stream named `name` (what [`AudioLink::name`] reports).
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            capture: false,
            on_event: None,
            sounds: Vec::new(),
        }
    }

    /// Short sounds the page preloads (`[("ding", "/sounds/ding.wav")]`),
    /// played by [`AudioOutput::sound`] in the queue's order. Any format
    /// the browser decodes; served by the application (an asset, an asset
    /// folder). Only the first channel is played.
    pub fn sounds<N: Into<String>, U: Into<String>>(
        mut self,
        sounds: impl IntoIterator<Item = (N, U)>,
    ) -> Self {
        self.sounds = sounds
            .into_iter()
            .map(|(n, u)| (n.into(), u.into()))
            .collect();
        self
    }

    /// Whether the microphone is on: each render starts or stops it. On,
    /// the browser asks for it (echo cancellation, noise suppression,
    /// automatic gain) and streams 20 ms frames to [`AudioLink::input`];
    /// off, every track is stopped and the browser's indicator goes off.
    pub fn capture(mut self, on: bool) -> Self {
        self.capture = on;
        self
    }

    /// The handler told what happens in the browser: decode it with
    /// [`parse_event`] from its `EventContext`.
    pub fn on_event(mut self, handler: HandlerSpec) -> Self {
        self.on_event = Some(handler);
        self
    }
}

/// The element that carries a page's audio stream.
pub trait AudioExt {
    /// Make this element the page's audio stream: the extension is hinted,
    /// and the socket's ticket rendered in.
    fn audio(self, audio: Audio) -> Self;
}

impl AudioExt for ElementBuilder {
    fn audio(self, audio: Audio) -> Self {
        let mut el = self
            .ext("audio")
            .attr("data-audio", &audio.name)
            .attr("data-audio-ws", AUDIO_WS_PATH)
            .attr("data-audio-capture", if audio.capture { "1" } else { "0" });
        if let Some(ticket) = rwire::ext_socket::ticket(AUDIO_WS_PATH, &audio.name) {
            el = el.attr("data-audio-ticket", &ticket);
        }
        if !audio.sounds.is_empty() {
            el = el.attr("data-audio-sounds", &sounds_json(&audio.sounds));
        }
        if let Some(handler) = audio.on_event {
            el = el.append([el_input()
                .attr("data-audio-report", "1")
                .attr("type", "hidden")
                .on(Ev::Change, handler)]);
        }
        el
    }
}

/// `[["name","url"],…]`, JSON-escaped.
fn sounds_json(sounds: &[(String, String)]) -> String {
    fn quote(s: &str) -> String {
        let mut out = String::with_capacity(s.len() + 2);
        out.push('"');
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }
    let items: Vec<String> = sounds
        .iter()
        .map(|(n, u)| format!("[{},{}]", quote(n), quote(u)))
        .collect();
    format!("[{}]", items.join(","))
}

fn el_input() -> ElementBuilder {
    rwire::el(El::Input)
}

/// Registers the audio socket on a server.
pub trait ServeAudio {
    /// Serve the audio socket: `on_audio` runs for each connection of a
    /// page's stream, with its [`AudioLink`]. When it returns, the socket
    /// is closed.
    fn serve_audio<H, Fut>(self, on_audio: H) -> Self
    where
        H: Fn(AudioLink) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static;
}

impl<F> ServeAudio for ServerWithRoot<F>
where
    F: Fn() -> ElementBuilder + Send + Sync + Clone + 'static,
{
    fn serve_audio<H, Fut>(self, on_audio: H) -> Self
    where
        H: Fn(AudioLink) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let on_audio = Arc::new(on_audio);
        self.asset(
            AUDIO_JS_PATH,
            "text/javascript; charset=utf-8",
            Cow::Borrowed(AUDIO_JS.as_bytes()),
        )
        .asset(
            WORKLET_JS_PATH,
            "text/javascript; charset=utf-8",
            Cow::Borrowed(WORKLET_JS.as_bytes()),
        )
        .ext_socket(AUDIO_WS_PATH, move |socket| {
            let on_audio = Arc::clone(&on_audio);
            async move { run_link(socket, move |link| on_audio(link)).await }
        })
    }
}

/// One connection of the audio socket: the next ticket first, then the
/// frames into the link's input and its output onto the socket, until the
/// page goes (the input ends) or the application's task returns (the
/// socket is closed).
async fn run_link<Fut>(socket: ExtSocket, on_audio: impl FnOnce(AudioLink) -> Fut)
where
    Fut: Future<Output = ()> + Send + 'static,
{
    let next = socket.next_ticket();
    let name = socket.label().to_string();
    let connection = socket.connection().clone();
    let server = Arc::clone(socket.shared());
    let (mut sink, mut stream) = socket.ws.split();
    let hello = wire::ToBrowser::Ticket(next.as_str().to_string()).encode();
    if sink.send(Message::Binary(hello)).await.is_err() {
        return;
    }
    let queue = Arc::new(link::InputQueue::default());
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let app = tokio::spawn(on_audio(AudioLink {
        input: AudioInput {
            queue: Arc::clone(&queue),
        },
        output: AudioOutput { tx },
        name,
        connection,
        server,
    }));
    let reader = async {
        while let Some(Ok(message)) = stream.next().await {
            match message {
                Message::Binary(data) => {
                    if let Some(frame) = Frame::decode(&data) {
                        queue.push(frame);
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    };
    let writer = async {
        while let Some(bytes) = rx.recv().await {
            if sink.send(Message::Binary(bytes)).await.is_err() {
                return;
            }
        }
        // Every output is gone: the application's task returned.
        let _ = sink.close().await;
    };
    tokio::select! {
        _ = reader => {}
        _ = writer => {}
    }
    queue.close();
    drop(next);
    // The task ends on its own once its input is over; a panic in it is
    // its own business, not the socket's.
    drop(app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rwire::{el, handler, State};

    #[derive(State, Default)]
    #[storage(memory)]
    struct Me {
        last: Option<AudioEvent>,
    }

    #[handler]
    fn heard(me: &mut Me, ctx: &rwire::EventContext) {
        me.last = parse_event(ctx);
    }

    #[test]
    fn the_render_drives_capture_and_carries_the_report_input() {
        let on = el(El::Div)
            .audio(Audio::new("voice").capture(true).on_event(heard()))
            .to_static_html();
        assert!(on.contains(r#"data-audio="voice""#), "{on}");
        assert!(on.contains(r#"data-audio-capture="1""#), "{on}");
        assert!(on.contains(r#"data-audio-ws="/_rw/ext/audio/ws""#), "{on}");
        assert!(on.contains("data-audio-report"), "{on}");
        assert!(
            !on.contains("data-audio-ticket"),
            "no ticket outside a page connection"
        );
        assert_eq!(
            sounds_json(&[
                ("ding".into(), "/s/ding.wav".into()),
                ("q\"\\".into(), "\n".into())
            ]),
            r#"[["ding","/s/ding.wav"],["q\"\\","\u000a"]]"#
        );
        let off = el(El::Div).audio(Audio::new("voice")).to_static_html();
        assert!(off.contains(r#"data-audio-capture="0""#), "{off}");
        assert!(
            !off.contains("data-audio-report"),
            "no handler, no report input"
        );
    }
}
