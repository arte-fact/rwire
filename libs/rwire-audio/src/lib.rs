//! Microphone capture and gapless playback for rwire.
//!
//! The audio of a page does not ride the page's socket: it has a socket of
//! its own, `/_rw/ext/audio/ws` ([`AUDIO_WS_PATH`]), registered by
//! [`ServeAudio::serve_audio`] and opened by the runtime extension only on
//! a page that renders an [`Audio`] element. The socket is authenticated by
//! a single-use ticket the render issues, bound to the page's session and
//! identity (see `rwire::ext_socket`); each connection hands the extension
//! the ticket of the next one, so a lost socket reconnects by itself.
//!
//! ```ignore
//! use rwire_audio::{Audio, AudioExt, AudioLink, ServeAudio};
//!
//! fn page(me: &Me) -> ElementBuilder {
//!     el(El::Div).audio(Audio::new("voice"))
//! }
//!
//! Server::bind(addr)?
//!     .root(root)
//!     .serve_audio(|mut link: AudioLink| async move {
//!         while let Some(frame) = link.input.recv().await {
//!             link.output.pcm(&frame.samples, 16_000); // an echo
//!         }
//!     })
//!     .run()
//!     .await
//! ```

mod link;
pub mod wire;

use std::future::Future;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use rwire::ext_socket::tokio_tungstenite::tungstenite::Message;
use rwire::{ElementBuilder, ExtSocket, ServerWithRoot};
use tokio::sync::mpsc;

pub use link::{AudioInput, AudioLink, AudioOutput, INPUT_CAPACITY};
pub use wire::{Frame, FRAME_SAMPLES, RATE};

/// The audio socket's path.
pub const AUDIO_WS_PATH: &str = "/_rw/ext/audio/ws";

/// A page's audio stream, rendered with [`AudioExt::audio`]. One per page.
pub struct Audio {
    name: String,
}

impl Audio {
    /// A stream named `name` (what [`AudioLink::name`] reports).
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
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
            .attr("data-audio-ws", AUDIO_WS_PATH);
        if let Some(ticket) = rwire::ext_socket::ticket(AUDIO_WS_PATH, &audio.name) {
            el = el.attr("data-audio-ticket", &ticket);
        }
        el
    }
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
        self.ext_socket(AUDIO_WS_PATH, move |socket| {
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
