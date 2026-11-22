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
#[derive(Clone, Debug, PartialEq)]
pub enum AudioEvent {
    /// The microphone's state, after each `capture(true)`.
    Capture(CaptureState),
    /// The output's state, when it changes.
    Output(OutputState),
}

impl AudioEvent {
    /// The event a report line carries, if it is one.
    pub fn parse(line: &str) -> Option<Self> {
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
        for bad in ["", "capture", "capture maybe", "capture granted now", "x y"] {
            assert_eq!(AudioEvent::parse(bad), None, "{bad:?}");
        }
    }
}
