//! Decoder for incoming client events.

use super::opcodes::*;

/// An event received from the client.
#[derive(Debug, Clone)]
pub struct ClientEvent {
    /// The handler index to invoke
    pub handler_idx: u8,
    /// The event type (click, input, etc.)
    pub event_type: u8,
    /// The element ref that triggered the event
    pub target_ref: u8,
    /// Optional payload data
    pub payload: Vec<u8>,
}

impl ClientEvent {
    /// Decode a client event from binary data.
    /// Format: [handler_idx, event_type, target_ref, payload_len, ...payload]
    pub fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 4 {
            return Err(DecodeError::TooShort);
        }

        let handler_idx = data[0];
        let event_type = data[1];
        let target_ref = data[2];
        let payload_len = data[3] as usize;

        if data.len() < 4 + payload_len {
            return Err(DecodeError::PayloadTruncated);
        }

        let payload = data[4..4 + payload_len].to_vec();

        Ok(Self {
            handler_idx,
            event_type,
            target_ref,
            payload,
        })
    }

    /// Get event type as a human-readable string.
    pub fn event_type_name(&self) -> &'static str {
        match self.event_type {
            EV_CLICK => "click",
            EV_DBLCLICK => "dblclick",
            EV_MOUSEDOWN => "mousedown",
            EV_MOUSEUP => "mouseup",
            EV_MOUSEMOVE => "mousemove",
            EV_SUBMIT => "submit",
            EV_INPUT => "input",
            EV_CHANGE => "change",
            EV_KEYDOWN => "keydown",
            EV_KEYUP => "keyup",
            EV_FOCUS => "focus",
            EV_BLUR => "blur",
            _ => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub enum DecodeError {
    TooShort,
    PayloadTruncated,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::TooShort => write!(f, "message too short"),
            DecodeError::PayloadTruncated => write!(f, "payload truncated"),
        }
    }
}

impl std::error::Error for DecodeError {}
