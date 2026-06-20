//! Decoder for incoming client events.

use super::opcodes::Ev;
use super::varint::read_varint;

/// Maximum allowed payload size (64KB). Prevents memory exhaustion from malicious inputs.
const MAX_PAYLOAD_SIZE: usize = 65_536;

/// An event received from the client.
#[derive(Debug, Clone)]
pub struct ClientEvent {
    /// The handler index to invoke
    pub handler_idx: u32,
    /// The event type (click, input, etc.)
    pub event_type: u8,
    /// The element ref that triggered the event
    pub target_ref: u8,
    /// Optional payload data (JSON for form/text events)
    pub payload: Vec<u8>,
    /// Handler parameter bytes (for ItemRef-based handlers)
    pub param_bytes: Vec<u8>,
}

impl ClientEvent {
    /// Decode a client event from binary data.
    ///
    /// Format (`payload_len` is a varint, so payloads may exceed 255 bytes; `param_len`
    /// is a single byte, as handler params are capped at 255):
    /// - [flags, handler_varint, event_type, target_ref, payload_len_varint, ...payload]
    /// - With params (flags & 0x80):
    ///   [flags, handler_varint, event_type, target_ref, param_len, ...params, payload_len_varint, ...payload]
    pub fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 4 {
            return Err(DecodeError::TooShort);
        }

        let flags = data[0];
        let has_params = (flags & 0x80) != 0;

        let (handler_idx, handler_len) = read_varint(&data[1..])
            .ok_or(DecodeError::TooShort)?;

        let mut pos = 1 + handler_len;
        if data.len() < pos + 2 {
            return Err(DecodeError::TooShort);
        }

        let event_type = data[pos];
        let target_ref = data[pos + 1];
        pos += 2;

        if has_params {
            // Extended format with param bytes
            if data.len() < pos + 1 {
                return Err(DecodeError::PayloadTruncated);
            }
            let param_len = data[pos] as usize;
            pos += 1;

            if data.len() < pos + param_len + 1 {
                return Err(DecodeError::PayloadTruncated);
            }

            let param_bytes = data[pos..pos + param_len].to_vec();
            pos += param_len;

            let (payload_len, len_bytes) =
                read_varint(&data[pos..]).ok_or(DecodeError::PayloadTruncated)?;
            let payload_len = payload_len as usize;
            pos += len_bytes;

            if payload_len > MAX_PAYLOAD_SIZE {
                return Err(DecodeError::PayloadTooLarge);
            }

            if data.len() < pos + payload_len {
                return Err(DecodeError::PayloadTruncated);
            }

            let payload = data[pos..pos + payload_len].to_vec();

            Ok(Self {
                handler_idx,
                event_type,
                target_ref,
                payload,
                param_bytes,
            })
        } else {
            // Standard format without params
            if data.len() < pos + 1 {
                return Err(DecodeError::PayloadTruncated);
            }
            let (payload_len, len_bytes) =
                read_varint(&data[pos..]).ok_or(DecodeError::PayloadTruncated)?;
            let payload_len = payload_len as usize;
            pos += len_bytes;

            if payload_len > MAX_PAYLOAD_SIZE {
                return Err(DecodeError::PayloadTooLarge);
            }

            if data.len() < pos + payload_len {
                return Err(DecodeError::PayloadTruncated);
            }

            let payload = data[pos..pos + payload_len].to_vec();

            Ok(Self {
                handler_idx,
                event_type,
                target_ref,
                payload,
                param_bytes: Vec::new(),
            })
        }
    }

    /// Get event type as a human-readable string.
    pub fn event_type_name(&self) -> &'static str {
        Ev::try_from(self.event_type)
            .map(|ev| ev.name())
            .unwrap_or("unknown")
    }
}

#[derive(Debug, Clone)]
pub enum DecodeError {
    TooShort,
    PayloadTruncated,
    PayloadTooLarge,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::TooShort => write!(f, "message too short"),
            DecodeError::PayloadTruncated => write!(f, "payload truncated"),
            DecodeError::PayloadTooLarge => write!(f, "payload too large"),
        }
    }
}

impl std::error::Error for DecodeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::write_varint;

    fn varint(value: u32) -> Vec<u8> {
        let mut out = Vec::new();
        write_varint(&mut out, value);
        out
    }

    #[test]
    fn decodes_payload_longer_than_255_bytes() {
        // Regression: the payload length is a varint, so a form field over 255 bytes
        // (e.g. a long chat message) round-trips fully. A single-byte length wrapped
        // (430 -> 174), truncating the JSON so the field parse failed and the send was
        // silently dropped — the composer appeared to do nothing.
        let payload = vec![b'x'; 430];
        let mut data = vec![0u8]; // flags: no params
        data.extend(varint(0)); // handler idx
        data.push(7); // event_type
        data.push(0); // target_ref
        data.extend(varint(payload.len() as u32));
        data.extend_from_slice(&payload);

        let ev = ClientEvent::decode(&data).expect("decodes");
        assert_eq!(ev.payload, payload);
    }

    #[test]
    fn decodes_short_payload_unchanged() {
        // A <=127-byte payload encodes its length in one varint byte, identical to the
        // old single-byte format — so existing short events are unaffected.
        let payload = b"hello".to_vec();
        let mut data = vec![0u8];
        data.extend(varint(3)); // handler idx
        data.push(7);
        data.push(0);
        data.extend(varint(payload.len() as u32));
        data.extend_from_slice(&payload);

        let ev = ClientEvent::decode(&data).expect("decodes");
        assert_eq!(ev.handler_idx, 3);
        assert_eq!(ev.event_type, 7);
        assert_eq!(ev.payload, payload);
    }
}
