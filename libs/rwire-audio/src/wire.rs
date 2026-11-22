//! What travels on the audio socket, in binary WebSocket messages. The
//! first byte says what a message is; numbers are little-endian.
//!
//! From the browser:
//!
//! | Byte | Rest | Meaning |
//! |---|---|---|
//! | `0x01` | `seq: u32`, `t: f64`, 320 × `i16` | A 20 ms frame of the microphone, 16 kHz mono PCM16; `t` in seconds on the page's audio clock |
//!
//! From the server:
//!
//! | Byte | Rest | Meaning |
//! |---|---|---|
//! | `0x80` | ticket (UTF-8) | The ticket of the next connection, sent first on each one |
//! | `0x81` | `rate: u32`, n × `i16` | PCM to queue, at `rate` Hz (the browser resamples) |
//! | `0x82` | id (UTF-8) | A mark: reported when the sample before it is played |
//! | `0x83` | name (UTF-8) | A preloaded sound, queued after what is already queued |
//! | `0x84` | — | Stop now: the queue is dropped |
//! | `0x85` | — | End of the utterance: playback ends once the queue is played |

/// The microphone's rate.
pub const RATE: u32 = 16_000;
/// Samples in a frame: 20 ms at 16 kHz.
pub const FRAME_SAMPLES: usize = 320;

const FRAME: u8 = 0x01;
const TICKET: u8 = 0x80;
const PCM: u8 = 0x81;
const MARK: u8 = 0x82;
const SOUND: u8 = 0x83;
const STOP: u8 = 0x84;
const END: u8 = 0x85;

/// The length of an encoded frame.
pub const FRAME_BYTES: usize = 1 + 4 + 8 + FRAME_SAMPLES * 2;

/// 20 ms of the microphone.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// Its number, counted from 0 on each capture start of the page: a gap
    /// says frames were dropped (see [`crate::INPUT_CAPACITY`]).
    pub seq: u32,
    /// When its first sample was captured, in seconds on the page's audio
    /// clock (the clock of every playback event too).
    pub timestamp: f64,
    /// Mono PCM16 at 16 kHz.
    pub samples: [i16; FRAME_SAMPLES],
}

impl Frame {
    /// The message carrying it.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(FRAME_BYTES);
        out.push(FRAME);
        out.extend_from_slice(&self.seq.to_le_bytes());
        out.extend_from_slice(&self.timestamp.to_le_bytes());
        for s in self.samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }

    /// The frame a message carries, if it is one.
    pub fn decode(data: &[u8]) -> Option<Frame> {
        if data.len() != FRAME_BYTES || data[0] != FRAME {
            return None;
        }
        let seq = u32::from_le_bytes(data[1..5].try_into().ok()?);
        let timestamp = f64::from_le_bytes(data[5..13].try_into().ok()?);
        let mut samples = [0i16; FRAME_SAMPLES];
        for (s, b) in samples.iter_mut().zip(data[13..].as_chunks::<2>().0) {
            *s = i16::from_le_bytes(*b);
        }
        Some(Frame {
            seq,
            timestamp,
            samples,
        })
    }
}

/// A message from the server.
#[derive(Clone, Debug, PartialEq)]
pub enum ToBrowser {
    /// The next connection's ticket.
    Ticket(String),
    /// PCM to queue, at `rate` Hz.
    Pcm { rate: u32, samples: Vec<i16> },
    /// A mark.
    Mark(String),
    /// A preloaded sound.
    Sound(String),
    /// Stop now.
    Stop,
    /// End of the utterance.
    End,
}

impl ToBrowser {
    /// The message's bytes.
    pub fn encode(&self) -> Vec<u8> {
        let tagged = |tag: u8, s: &str| {
            let mut out = Vec::with_capacity(1 + s.len());
            out.push(tag);
            out.extend_from_slice(s.as_bytes());
            out
        };
        match self {
            Self::Ticket(t) => tagged(TICKET, t),
            Self::Pcm { rate, samples } => {
                let mut out = Vec::with_capacity(5 + samples.len() * 2);
                out.push(PCM);
                out.extend_from_slice(&rate.to_le_bytes());
                for s in samples {
                    out.extend_from_slice(&s.to_le_bytes());
                }
                out
            }
            Self::Mark(id) => tagged(MARK, id),
            Self::Sound(name) => tagged(SOUND, name),
            Self::Stop => vec![STOP],
            Self::End => vec![END],
        }
    }

    /// The message `data` carries, if it is one.
    pub fn decode(data: &[u8]) -> Option<Self> {
        let (&tag, rest) = data.split_first()?;
        let text = || String::from_utf8(rest.to_vec()).ok();
        Some(match tag {
            TICKET => Self::Ticket(text()?),
            PCM => {
                if rest.len() < 4 || rest.len() % 2 != 0 {
                    return None;
                }
                let rate = u32::from_le_bytes(rest[..4].try_into().ok()?);
                let samples = rest[4..]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| i16::from_le_bytes(*b))
                    .collect();
                Self::Pcm { rate, samples }
            }
            MARK => Self::Mark(text()?),
            SOUND => Self::Sound(text()?),
            STOP if rest.is_empty() => Self::Stop,
            END if rest.is_empty() => Self::End,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_round_trips_and_has_a_fixed_size() {
        let mut samples = [0i16; FRAME_SAMPLES];
        samples[0] = i16::MIN;
        samples[319] = i16::MAX;
        samples[7] = -2;
        let f = Frame {
            seq: 0xDEAD_BEEF,
            timestamp: 12.345,
            samples,
        };
        let bytes = f.encode();
        assert_eq!(bytes.len(), FRAME_BYTES);
        assert_eq!(bytes.len(), 653);
        assert_eq!(Frame::decode(&bytes), Some(f));
        assert_eq!(Frame::decode(&bytes[..652]), None);
        let mut other = bytes.clone();
        other[0] = 0x02;
        assert_eq!(Frame::decode(&other), None);
    }

    #[test]
    fn server_messages_round_trip() {
        for m in [
            ToBrowser::Ticket("rwt_x".into()),
            ToBrowser::Pcm {
                rate: 24_000,
                samples: vec![1, -1, i16::MAX],
            },
            ToBrowser::Mark("s1".into()),
            ToBrowser::Sound("ding".into()),
            ToBrowser::Stop,
            ToBrowser::End,
        ] {
            assert_eq!(ToBrowser::decode(&m.encode()), Some(m));
        }
        assert_eq!(ToBrowser::decode(&[0x81, 1, 0, 0]), None);
        assert_eq!(ToBrowser::decode(&[0x84, 0]), None);
        assert_eq!(ToBrowser::decode(&[]), None);
    }
}
