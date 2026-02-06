//! Decoder for incoming client events.

use super::opcodes::Ev;

/// Maximum allowed payload size (64KB). Prevents memory exhaustion from malicious inputs.
const MAX_PAYLOAD_SIZE: usize = 65_536;

/// An event received from the client.
#[derive(Debug, Clone)]
pub struct ClientEvent {
    /// The handler index to invoke
    pub handler_idx: u8,
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
    /// Supports two formats:
    /// - Legacy: [handler_idx, event_type, target_ref, payload_len, ...payload]
    /// - With params: [handler_idx, event_type, target_ref, param_len, ...param_bytes, payload_len, ...payload]
    ///
    /// The format is determined by checking if the message has the extended flag (0x80) set
    /// on the handler_idx byte. If set, param_bytes are included.
    pub fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 4 {
            return Err(DecodeError::TooShort);
        }

        let first_byte = data[0];
        let has_params = (first_byte & 0x80) != 0;
        let handler_idx = first_byte & 0x7F;
        let event_type = data[1];
        let target_ref = data[2];

        if has_params {
            // Extended format with param bytes
            let param_len = data[3] as usize;
            if data.len() < 4 + param_len + 1 {
                return Err(DecodeError::PayloadTruncated);
            }

            let param_bytes = data[4..4 + param_len].to_vec();
            let payload_start = 4 + param_len;
            let payload_len = data[payload_start] as usize;

            if payload_len > MAX_PAYLOAD_SIZE {
                return Err(DecodeError::PayloadTooLarge);
            }

            if data.len() < payload_start + 1 + payload_len {
                return Err(DecodeError::PayloadTruncated);
            }

            let payload = data[payload_start + 1..payload_start + 1 + payload_len].to_vec();

            Ok(Self {
                handler_idx,
                event_type,
                target_ref,
                payload,
                param_bytes,
            })
        } else {
            // Legacy format without params
            let payload_len = data[3] as usize;

            if payload_len > MAX_PAYLOAD_SIZE {
                return Err(DecodeError::PayloadTooLarge);
            }

            if data.len() < 4 + payload_len {
                return Err(DecodeError::PayloadTruncated);
            }

            let payload = data[4..4 + payload_len].to_vec();

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
