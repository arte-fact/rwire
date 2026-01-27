//! Binary protocol for RustWire DOM opcodes and events.

pub mod decoder;
pub mod encoder;
pub mod opcodes;

pub use decoder::{ClientEvent, DecodeError};
pub use encoder::OpcodeBuffer;
pub use opcodes::*;
pub use opcodes::{El, Ev};
