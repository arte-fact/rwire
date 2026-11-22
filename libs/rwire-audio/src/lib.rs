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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use rwire::ext_socket::tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use rwire::{El, ElementBuilder, Ev, ExtSocket, HandlerSpec, ServerWithRoot};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;

pub use event::{parse_event, AudioEvent, CaptureState, OutputState};
pub use link::{
    AudioInput, AudioLink, AudioOutput, OutputError, INPUT_CAPACITY, MAX_RATE, MIN_RATE,
    OUTPUT_BUDGET,
};
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
    /// page's stream, with its [`AudioLink`].
    ///
    /// **The task's life is the link's.**
    /// - While the link lives, the task is never stopped, however long it
    ///   waits: no frame for minutes (`capture(false)`) is not a reason.
    /// - The link closes when the browser closes or loses the socket, when
    ///   the socket stays silent through a ping ([`KEEPALIVE`] twice),
    ///   when one write takes longer than [`SEND_TIMEOUT`], or when the
    ///   page connection that rendered it ends. Then `input.recv()`
    ///   returns `None` and `output.closed()` resolves: the task should
    ///   return. One still running [`ABORT_GRACE`] after the close is
    ///   aborted.
    /// - When the task returns, the socket is closed (the extension
    ///   reconnects and a new task starts).
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

/// Silence on the socket (no frame, no pong) after which it is pinged, and
/// after which, pinged and still silent, it is closed: a half-open socket
/// lasts at most twice this. As the page socket.
pub const KEEPALIVE: Duration = Duration::from_secs(30);
/// How long one message may take to be written to the socket: a browser
/// that stops reading gets its link closed.
pub const SEND_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the application's task may run on once its link is closed,
/// before it is aborted (see [`ServeAudio::serve_audio`]).
pub const ABORT_GRACE: Duration = Duration::from_secs(5);

/// One connection of the audio socket.
async fn run_link<Fut>(socket: ExtSocket, on_audio: impl FnOnce(AudioLink) -> Fut)
where
    Fut: Future<Output = ()> + Send + 'static,
{
    let next = socket.next_ticket();
    let name = socket.label().to_string();
    let connection = socket.connection().clone();
    let server = Arc::clone(socket.shared());
    let page_closed = socket.page_closed();
    let hello = next.as_str().to_string();
    tokio::select! {
        _ = drive_link(socket.ws, hello, |input, output| {
            on_audio(AudioLink {
                input,
                output,
                name,
                connection,
                server,
            })
        }) => {}
        // The page connection that issued the ticket is gone.
        _ = page_closed => {}
    }
    // `next` expires from now (TICKET_TTL), or with its page connection.
    drop(next);
}

/// The link over `ws`: the next ticket first, then the frames into the
/// input and the output onto the socket, with a keepalive and a send
/// timeout, until the socket ends, times out, or the task returns. A task
/// still running once the socket is gone is aborted after [`ABORT_GRACE`].
async fn drive_link<S, Fut>(
    ws: WebSocketStream<S>,
    hello: String,
    app: impl FnOnce(AudioInput, AudioOutput) -> Fut,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (mut sink, stream) = ws.split();
    let hello = wire::ToBrowser::Ticket(hello).encode();
    if sink.send(Message::Binary(hello)).await.is_err() {
        return;
    }
    let queue = Arc::new(link::InputQueue::default());
    let (tx, rx) = mpsc::unbounded_channel::<Message>();
    let output = AudioOutput::new(tx.clone());
    let pending = Arc::clone(&output.pending);
    let mut task = tokio::spawn(app(
        AudioInput {
            queue: Arc::clone(&queue),
        },
        output,
    ));
    tokio::select! {
        _ = read_loop(stream, &queue, tx, KEEPALIVE) => {}
        _ = write_loop(sink, rx, &pending, SEND_TIMEOUT) => {}
        _ = &mut task => {}
    }
    // The socket is gone (the writer and its receiver are dropped: the
    // output is closed), and so is the input once it is read.
    queue.close();
    if !task.is_finished() {
        tokio::spawn(abort_after(task, ABORT_GRACE));
    }
}

/// Frames into `queue` until the socket ends or stays silent through a
/// ping (`keepalive` twice).
async fn read_loop<St, E>(
    mut stream: St,
    queue: &link::InputQueue,
    pings: mpsc::UnboundedSender<Message>,
    keepalive: Duration,
) where
    St: futures::Stream<Item = Result<Message, E>> + Unpin,
{
    let mut pinged = false;
    loop {
        match tokio::time::timeout(keepalive, stream.next()).await {
            Ok(Some(Ok(message))) => {
                pinged = false;
                match message {
                    Message::Binary(data) => {
                        if let Some(frame) = Frame::decode(&data) {
                            queue.push(frame);
                        }
                    }
                    Message::Close(_) => return,
                    _ => {}
                }
            }
            Ok(_) => return,
            Err(_) if pinged => return,
            Err(_) => {
                if pings.send(Message::Ping(Vec::new())).is_err() {
                    return;
                }
                pinged = true;
            }
        }
    }
}

/// The output onto the socket; ends when a write fails or takes longer
/// than `limit`.
async fn write_loop<Si>(
    mut sink: Si,
    mut rx: mpsc::UnboundedReceiver<Message>,
    pending: &AtomicUsize,
    limit: Duration,
) where
    Si: futures::Sink<Message> + Unpin,
{
    while let Some(message) = rx.recv().await {
        let len = match &message {
            Message::Binary(b) => b.len(),
            _ => 0,
        };
        match tokio::time::timeout(limit, sink.send(message)).await {
            Ok(Ok(())) => {
                pending.fetch_sub(len, Ordering::AcqRel);
            }
            _ => return,
        }
    }
}

/// A task still running after its link closed: `grace` more, then aborted.
async fn abort_after(mut task: tokio::task::JoinHandle<()>, grace: Duration) {
    if tokio::time::timeout(grace, &mut task).await.is_err() {
        task.abort();
    }
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

/// The output's limits, the socket's timeouts, and the task's life.
#[cfg(test)]
mod link_tests {
    use super::*;
    use rwire::ext_socket::tokio_tungstenite::{accept_async, client_async};
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::time::Instant;

    #[test]
    fn the_output_refuses_past_its_budget_a_bad_rate_and_once_closed() {
        let (tx, rx) = mpsc::unbounded_channel();
        let out = AudioOutput::new(tx);
        let second = vec![0i16; 24_000];
        let mut queued = 0;
        let refused = loop {
            match out.pcm(&second, 24_000) {
                Ok(()) => queued += 1,
                Err(e) => break e,
            }
            assert!(queued <= 120, "never refused");
        };
        assert_eq!(refused, OutputError::QueueFull);
        // 60 s at 24 kHz, less the 5-byte header of each message.
        assert_eq!(queued, 59, "a minute at 24 kHz fills the budget");
        assert!(out.pending.load(Ordering::Acquire) <= OUTPUT_BUDGET);
        // Marks, sounds and the end count too (one bigger than what is left).
        assert_eq!(out.mark(&"m".repeat(60_000)), Err(OutputError::QueueFull));
        out.stop(); // never refused for a full queue
                    // The writer drains: room again.
        out.pending.store(0, Ordering::Release);
        assert_eq!(out.pcm(&second, 24_000), Ok(()));
        for bad in [0, 1, 7_999, 192_001, u32::MAX] {
            assert_eq!(
                out.pcm(&[0; 10], bad),
                Err(OutputError::RateRefused),
                "{bad}"
            );
        }
        for good in [8_000, 16_000, 22_050, 192_000] {
            assert_eq!(out.pcm(&[0; 10], good), Ok(()), "{good}");
        }
        drop(rx);
        assert_eq!(out.pcm(&[0; 10], 24_000), Err(OutputError::Closed));
        assert_eq!(out.sound("ding"), Err(OutputError::Closed));
        assert_eq!(out.end(), Err(OutputError::Closed));
    }

    /// A socket that never takes anything.
    struct Stalled;
    impl futures::Sink<Message> for Stalled {
        type Error = ();
        fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), ()>> {
            Poll::Pending
        }
        fn start_send(self: Pin<&mut Self>, _: Message) -> Result<(), ()> {
            Ok(())
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), ()>> {
            Poll::Pending
        }
        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), ()>> {
            Poll::Pending
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_write_that_never_completes_closes_the_link_after_the_send_timeout() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(Message::Binary(vec![1])).unwrap();
        let start = Instant::now();
        let pending = AtomicUsize::new(1);
        let bounded = tokio::time::timeout(
            Duration::from_secs(3600),
            write_loop(Stalled, rx, &pending, SEND_TIMEOUT),
        );
        assert!(bounded.await.is_ok(), "the write never gave up");
        assert_eq!(start.elapsed(), SEND_TIMEOUT);
    }

    #[tokio::test(start_paused = true)]
    async fn a_silent_socket_is_pinged_then_closed() {
        let (pings, mut sent) = mpsc::unbounded_channel();
        let start = Instant::now();
        let silent = futures::stream::pending::<Result<Message, ()>>();
        let queue = link::InputQueue::default();
        let bounded = tokio::time::timeout(
            Duration::from_secs(3600),
            read_loop(silent, &queue, pings, KEEPALIVE),
        );
        assert!(bounded.await.is_ok(), "the silent socket was kept forever");
        assert_eq!(
            start.elapsed(),
            KEEPALIVE * 2,
            "pinged after one, closed after two"
        );
        assert!(matches!(sent.try_recv(), Ok(Message::Ping(_))));
        assert!(sent.try_recv().is_err(), "one ping");
    }

    /// A link over an in-memory socket, and the browser's end of it, which
    /// reads (answering pings) until dropped.
    async fn memory_link(
        app: impl FnOnce(AudioInput, AudioOutput) -> futures::future::BoxFuture<'static, ()>
            + Send
            + 'static,
    ) -> tokio::task::JoinHandle<()> {
        let (server_io, client_io) = tokio::io::duplex(1 << 16);
        let server = tokio::spawn(async move {
            let ws = accept_async(server_io).await.unwrap();
            drive_link(ws, "rwt_next".into(), app).await;
        });
        let (mut ws, _) = client_async("ws://test/", client_io).await.unwrap();
        tokio::spawn(async move { while let Some(Ok(_)) = ws.next().await {} });
        server
    }

    /// Says when it is dropped (its task ended or was aborted).
    struct Alive(mpsc::UnboundedSender<Instant>);
    impl Drop for Alive {
        fn drop(&mut self) {
            let _ = self.0.send(Instant::now());
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_task_waiting_on_a_live_link_is_never_stopped_however_long_it_waits() {
        let (tx, mut dropped) = mpsc::unbounded_channel();
        let _link = memory_link(move |mut input, _output| {
            Box::pin(async move {
                let _alive = Alive(tx);
                // capture(false): no frame for minutes.
                while input.recv().await.is_some() {}
            })
        })
        .await;
        tokio::time::sleep(Duration::from_secs(600)).await;
        assert!(dropped.try_recv().is_err(), "the task lives with its link");
    }

    #[tokio::test(start_paused = true)]
    async fn once_the_link_closes_input_ends_closed_resolves_and_a_lingering_task_is_aborted_after_the_grace(
    ) {
        // A task that returns on close: recv() gives None, closed() resolves.
        let (tx, mut seen) = mpsc::unbounded_channel();
        let (server_io, client_io) = tokio::io::duplex(1 << 16);
        let server = tokio::spawn(async move {
            let ws = accept_async(server_io).await.unwrap();
            drive_link(ws, "rwt_next".into(), move |mut input, output| async move {
                while input.recv().await.is_some() {}
                let _ = tx.send("recv: None");
                output.closed().await;
                let _ = tx.send("closed()");
            })
            .await;
        });
        let (ws, _) = client_async("ws://test/", client_io).await.unwrap();
        drop(ws);
        server.await.unwrap();
        assert_eq!(seen.recv().await, Some("recv: None"));
        assert_eq!(seen.recv().await, Some("closed()"));

        // A task that ignores the close: aborted, but only after the grace.
        let (tx, mut dropped) = mpsc::unbounded_channel();
        let (server_io, client_io) = tokio::io::duplex(1 << 16);
        let server = tokio::spawn(async move {
            let ws = accept_async(server_io).await.unwrap();
            drive_link(ws, "rwt_next".into(), move |_input, _output| async move {
                let _alive = Alive(tx);
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            })
            .await;
        });
        let (mut ws, _) = client_async("ws://test/", client_io).await.unwrap();
        // The browser reads (and answers pings) until it goes.
        let browser = tokio::spawn(async move { while let Some(Ok(_)) = ws.next().await {} });
        tokio::time::sleep(Duration::from_secs(120)).await;
        assert!(dropped.try_recv().is_err(), "alive while the link is");
        let closed_at = Instant::now();
        browser.abort();
        server.await.unwrap();
        let aborted_at = tokio::time::timeout(Duration::from_secs(3600), dropped.recv())
            .await
            .expect("never aborted")
            .expect("aborted");
        assert_eq!(aborted_at - closed_at, ABORT_GRACE);
    }
}
