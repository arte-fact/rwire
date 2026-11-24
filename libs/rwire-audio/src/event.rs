//! What the browser tells the server about the stream, through the usual
//! handler path (`Audio::on_event`): one line per event in the value of the
//! stream's hidden report input.

use rwire::EventContext;

/// The microphone, as the browser sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureState {
    /// Granted: frames are flowing.
    Granted,
    /// The user (or a policy) refused it.
    Denied,
    /// No microphone, one already in use, or no Web Audio.
    Unavailable,
    /// The page is not a secure context (HTTPS or localhost): the browser
    /// offers no microphone at all.
    Insecure,
}

/// The loudspeaker side: whether the page's audio context can play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputState {
    /// Running: queued audio plays.
    Ready,
    /// Suspended by the autoplay policy until a user gesture in the page
    /// (any tap or key: the extension resumes the context inside it).
    Blocked,
    /// It ran, then the browser suspended it (a call, the device changed).
    Interrupted,
}

/// An event of a page's audio stream.
///
/// Every `at` is in seconds on the page's audio clock, the clock of
/// [`crate::Frame::timestamp`], at the moment the sample is **heard**: the
/// browser adds its output latency and reports the event at that time.
///
/// **These are the client's claims.** The browser reports them and nothing
/// stops a page from sending any line it likes: use them to drive the UI
/// and the conversation, never for authorization, billing or accounting.
#[derive(Clone, Debug, PartialEq)]
pub enum AudioEvent {
    /// The microphone's state, after each `capture(true)`.
    Capture(CaptureState),
    /// The output's state, when it changes.
    Output(OutputState),
    /// The first sample of an utterance (after an end or a stop) is heard.
    PlaybackStarted { at: f64 },
    /// Everything queued before `output.mark(id)` has been heard.
    MarkReached { id: String, at: f64 },
    /// The queue ran dry before `output.end()`: the utterance goes on when
    /// more PCM comes (no new `PlaybackStarted`).
    Underrun { at: f64 },
    /// Everything queued before `output.end()` has been heard.
    PlaybackEnded { at: f64 },
    /// `output.stop()` dropped the queue: silence from `at`. `last_mark` is
    /// the last mark reached in the utterance, and `samples_since` the PCM
    /// samples heard after it, counted at the rate they were sent (sounds
    /// not counted): where the reading stopped.
    Stopped {
        at: f64,
        last_mark: Option<String>,
        samples_since: u64,
    },
}

impl AudioEvent {
    /// The event a report line carries, if it is one.
    pub fn parse(line: &str) -> Option<Self> {
        if let Some(event) = Self::parse_playback(line) {
            return Some(event);
        }
        let mut words = line.split(' ');
        let kind = words.next()?;
        let arg = words.next()?;
        if words.next().is_some() {
            return None;
        }
        Some(match (kind, arg) {
            ("capture", "granted") => Self::Capture(CaptureState::Granted),
            ("capture", "denied") => Self::Capture(CaptureState::Denied),
            ("capture", "unavailable") => Self::Capture(CaptureState::Unavailable),
            ("capture", "insecure") => Self::Capture(CaptureState::Insecure),
            ("output", "ready") => Self::Output(OutputState::Ready),
            ("output", "blocked") => Self::Output(OutputState::Blocked),
            ("output", "interrupted") => Self::Output(OutputState::Interrupted),
            _ => return None,
        })
    }
}

impl AudioEvent {
    /// `<kind> <t> [rest]`: the playback events.
    fn parse_playback(line: &str) -> Option<Self> {
        let (kind, rest) = line.split_once(' ')?;
        let (t, rest) = match rest.split_once(' ') {
            Some((t, rest)) => (t, Some(rest)),
            None => (rest, None),
        };
        let at: f64 = t.parse().ok().filter(|t: &f64| t.is_finite())?;
        Some(match (kind, rest) {
            ("started", None) => Self::PlaybackStarted { at },
            ("underrun", None) => Self::Underrun { at },
            ("ended", None) => Self::PlaybackEnded { at },
            ("mark", Some(id)) => Self::MarkReached {
                id: id.to_string(),
                at,
            },
            ("stopped", Some(rest)) => {
                let (since, mark) = match rest.split_once(' ') {
                    Some((since, mark)) => (since, Some(mark.to_string())),
                    None => (rest, None),
                };
                Self::Stopped {
                    at,
                    last_mark: mark,
                    samples_since: since.parse().ok()?,
                }
            }
            _ => return None,
        })
    }
}

/// The event an `Audio::on_event` handler was called for.
pub fn parse_event(ctx: &EventContext) -> Option<AudioEvent> {
    ctx.text().and_then(AudioEvent::parse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_report_line_of_the_extension_parses() {
        use AudioEvent::*;
        for (line, event) in [
            ("capture granted", Capture(CaptureState::Granted)),
            ("capture denied", Capture(CaptureState::Denied)),
            ("capture unavailable", Capture(CaptureState::Unavailable)),
            ("capture insecure", Capture(CaptureState::Insecure)),
            ("output ready", Output(OutputState::Ready)),
            ("output blocked", Output(OutputState::Blocked)),
            ("output interrupted", Output(OutputState::Interrupted)),
        ] {
            assert_eq!(AudioEvent::parse(line), Some(event), "{line}");
        }
        for (line, event) in [
            ("started 1.250000", PlaybackStarted { at: 1.25 }),
            ("underrun 2.000000", Underrun { at: 2.0 }),
            ("ended 3.5", PlaybackEnded { at: 3.5 }),
            (
                "mark 1.75 sentence 1",
                MarkReached {
                    id: "sentence 1".into(),
                    at: 1.75,
                },
            ),
            (
                "stopped 4.2 6400 s 2",
                Stopped {
                    at: 4.2,
                    last_mark: Some("s 2".into()),
                    samples_since: 6400,
                },
            ),
            (
                "stopped 4.2 0",
                Stopped {
                    at: 4.2,
                    last_mark: None,
                    samples_since: 0,
                },
            ),
        ] {
            assert_eq!(AudioEvent::parse(line), Some(event), "{line}");
        }
        for bad in [
            "",
            "capture",
            "capture maybe",
            "capture granted now",
            "x y",
            "started",
            "started soon",
            "started NaN",
            "mark 1.0",
            "stopped 1.0",
            "stopped 1.0 many",
            "ended 1.0 extra",
        ] {
            assert_eq!(AudioEvent::parse(bad), None, "{bad:?}");
        }
    }
}
