//! The server side of a page's audio stream: [`AudioLink`].

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rwire::ext_socket::tokio_tungstenite::tungstenite::Message;

use rwire::{ConnectionInfo, Identity, SharedServerState};
use tokio::sync::{mpsc, Notify};

use crate::wire::{Frame, ToBrowser};

/// How many frames wait for the application at most: 50, one second. When
/// the application reads slower than the microphone, the oldest frames are
/// dropped and the gap shows in [`Frame::seq`].
pub const INPUT_CAPACITY: usize = 50;

/// One connection of a page's audio stream. A lost socket that reconnects
/// by itself gives a new link with the same [`Self::session`],
/// [`Self::name`] and [`Self::connection_id`].
pub struct AudioLink {
    /// The microphone's frames.
    pub input: AudioInput,
    /// What the page plays.
    pub output: AudioOutput,
    pub(crate) name: String,
    pub(crate) connection: ConnectionInfo,
    pub(crate) server: Arc<SharedServerState>,
}

impl AudioLink {
    /// The session of the page.
    pub fn session(&self) -> &str {
        &self.connection.session
    }

    /// The stream's name (`Audio::new(name)`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The page connection that rendered the stream: two tabs of one
    /// session have two.
    pub fn connection_id(&self) -> u64 {
        self.connection.connection_id
    }

    /// Who the page belongs to, under delegated auth.
    pub fn identity(&self) -> Option<&Identity> {
        self.connection.identity.as_ref()
    }

    /// The server's shared state: `link.server().update_shared::<T>(…)`
    /// changes a shared state and re-renders the pages that show it.
    pub fn server(&self) -> &Arc<SharedServerState> {
        &self.server
    }
}

#[derive(Default)]
struct Queue {
    frames: VecDeque<Frame>,
    closed: bool,
}

/// The frames waiting for the application (bounded, oldest dropped first).
#[derive(Default)]
pub(crate) struct InputQueue {
    queue: Mutex<Queue>,
    ready: Notify,
}

impl InputQueue {
    pub(crate) fn push(&self, frame: Frame) {
        {
            let mut q = self.queue.lock().unwrap_or_else(|e| e.into_inner());
            if q.frames.len() == INPUT_CAPACITY {
                q.frames.pop_front();
            }
            q.frames.push_back(frame);
        }
        self.ready.notify_one();
    }

    pub(crate) fn close(&self) {
        self.queue.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
        self.ready.notify_one();
    }
}

/// The microphone side of a link.
pub struct AudioInput {
    pub(crate) queue: Arc<InputQueue>,
}

impl AudioInput {
    /// The next frame; `None` once the socket is gone and the frames read.
    pub async fn recv(&mut self) -> Option<Frame> {
        loop {
            {
                let mut q = self.queue.queue.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(f) = q.frames.pop_front() {
                    return Some(f);
                }
                if q.closed {
                    return None;
                }
            }
            self.queue.ready.notified().await;
        }
    }
}

/// Why [`AudioOutput`] refused something.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputError {
    /// More than [`OUTPUT_BUDGET`] bytes wait to be sent: the browser does
    /// not read (a stalled or half-open socket). Nothing was queued; try
    /// again later, or `stop()`.
    QueueFull,
    /// The socket is gone: nothing will be played on this link.
    Closed,
    /// The rate is outside [`MIN_RATE`]..=[`MAX_RATE`]: refused, the
    /// browser could not play it.
    RateRefused,
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::QueueFull => "the audio output queue is full",
            Self::Closed => "the audio link is closed",
            Self::RateRefused => "the PCM rate is outside 8000..=192000 Hz",
        })
    }
}

impl std::error::Error for OutputError {}

/// How many bytes may wait to be sent to the browser: 30 s of 48 kHz PCM16
/// (60 s at 24 kHz). Past it, the output refuses with
/// [`OutputError::QueueFull`].
pub const OUTPUT_BUDGET: usize = 30 * 48_000 * 2;
/// The lowest PCM rate [`AudioOutput::pcm`] takes.
pub const MIN_RATE: u32 = 8_000;
/// The highest PCM rate [`AudioOutput::pcm`] takes.
pub const MAX_RATE: u32 = 192_000;

/// The loudspeaker side of a link. Everything is queued in call order and
/// played without gaps; nothing waits.
#[derive(Clone)]
pub struct AudioOutput {
    pub(crate) tx: mpsc::UnboundedSender<Message>,
    /// Bytes queued and not yet written to the socket.
    pub(crate) pending: Arc<AtomicUsize>,
}

impl AudioOutput {
    pub(crate) fn new(tx: mpsc::UnboundedSender<Message>) -> Self {
        Self {
            tx,
            pending: Arc::default(),
        }
    }

    fn send(&self, m: ToBrowser, budgeted: bool) -> Result<(), OutputError> {
        if self.tx.is_closed() {
            return Err(OutputError::Closed);
        }
        let bytes = m.encode();
        let len = bytes.len();
        if budgeted && self.pending.load(Ordering::Acquire) + len > OUTPUT_BUDGET {
            return Err(OutputError::QueueFull);
        }
        self.pending.fetch_add(len, Ordering::AcqRel);
        self.tx.send(Message::Binary(bytes)).map_err(|_| {
            self.pending.fetch_sub(len, Ordering::AcqRel);
            OutputError::Closed
        })
    }

    /// Queue mono PCM16 at `rate` Hz (24 000 for most TTS; the browser
    /// resamples to its own rate).
    ///
    /// # Errors
    ///
    /// [`OutputError::RateRefused`] for a rate outside
    /// [`MIN_RATE`]..=[`MAX_RATE`], [`OutputError::QueueFull`] past
    /// [`OUTPUT_BUDGET`], [`OutputError::Closed`] once the socket is gone.
    /// Nothing is queued then.
    pub fn pcm(&self, samples: &[i16], rate: u32) -> Result<(), OutputError> {
        if !(MIN_RATE..=MAX_RATE).contains(&rate) {
            return Err(OutputError::RateRefused);
        }
        self.send(
            ToBrowser::Pcm {
                rate,
                samples: samples.to_vec(),
            },
            true,
        )
    }

    /// Queue a mark: `MarkReached` is reported when everything queued
    /// before it has been heard.
    ///
    /// # Errors
    ///
    /// [`OutputError::QueueFull`] or [`OutputError::Closed`], as [`Self::pcm`].
    pub fn mark(&self, id: &str) -> Result<(), OutputError> {
        self.send(ToBrowser::Mark(id.to_string()), true)
    }

    /// Queue a preloaded sound (`Audio::sounds`), after what is queued.
    ///
    /// # Errors
    ///
    /// [`OutputError::QueueFull`] or [`OutputError::Closed`], as [`Self::pcm`].
    pub fn sound(&self, name: &str) -> Result<(), OutputError> {
        self.send(ToBrowser::Sound(name.to_string()), true)
    }

    /// Close the utterance: `PlaybackEnded` is reported once the queue is
    /// heard. An empty queue before it is an `Underrun`, not an end.
    ///
    /// # Errors
    ///
    /// [`OutputError::QueueFull`] or [`OutputError::Closed`], as [`Self::pcm`].
    pub fn end(&self) -> Result<(), OutputError> {
        self.send(ToBrowser::End, true)
    }

    /// Drop the queue and stop at once (`Stopped` is reported). Never
    /// refused for a full queue; a no-op once the socket is gone.
    pub fn stop(&self) {
        let _ = self.send(ToBrowser::Stop, false);
    }

    /// Whether the socket is still there to take what is queued.
    pub fn is_open(&self) -> bool {
        !self.tx.is_closed()
    }

    /// Resolves once the socket is gone (closed by the browser, lost, or
    /// timed out). Then [`AudioInput::recv`] returns `None` too.
    pub async fn closed(&self) {
        self.tx.closed().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::FRAME_SAMPLES;

    fn frame(seq: u32) -> Frame {
        Frame {
            seq,
            timestamp: seq as f64 * 0.02,
            samples: [0; FRAME_SAMPLES],
        }
    }

    #[tokio::test]
    async fn a_slow_reader_loses_the_oldest_frames_and_sees_the_gap() {
        let queue = Arc::new(InputQueue::default());
        let mut input = AudioInput {
            queue: Arc::clone(&queue),
        };
        for seq in 0..(INPUT_CAPACITY as u32 + 10) {
            queue.push(frame(seq));
        }
        queue.close();
        let mut seqs = Vec::new();
        while let Some(f) = input.recv().await {
            seqs.push(f.seq);
        }
        assert_eq!(seqs.len(), INPUT_CAPACITY);
        assert_eq!(seqs[0], 10, "the 10 oldest are gone");
        assert_eq!(*seqs.last().unwrap(), INPUT_CAPACITY as u32 + 9);
    }

    #[tokio::test]
    async fn recv_waits_for_a_frame() {
        let queue = Arc::new(InputQueue::default());
        let mut input = AudioInput {
            queue: Arc::clone(&queue),
        };
        let q = Arc::clone(&queue);
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            q.push(frame(3));
        });
        assert_eq!(input.recv().await.map(|f| f.seq), Some(3));
    }
}
