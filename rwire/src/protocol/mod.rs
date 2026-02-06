//! Binary protocol for RustWire DOM opcodes and events.

pub mod decoder;
pub mod encoder;
pub mod opcodes;
pub mod varint;

pub use decoder::{ClientEvent, DecodeError};
pub use encoder::OpcodeBuffer;
pub use opcodes::*;
pub use varint::{read_varint, write_varint, VARINT_JS, VARINT_MAX};
