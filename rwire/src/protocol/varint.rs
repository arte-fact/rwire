//! Variable-length integer encoding for element references.
//!
//! Allows support for >255 elements per message using a compact encoding:
//! - 0x00-0x7F: Single byte (0-127)
//! - 0x80-0xBF: Two bytes (128-16,511)
//! - 0xC0-0xFF: Three bytes (16,512-1,048,575)
//!
//! This provides backward compatibility for simple UIs while supporting
//! complex UIs with thousands of elements.

use bytes::BufMut;

/// Write a varint-encoded value to a buffer.
///
/// Returns the number of bytes written (1-3).
pub fn write_varint<B: BufMut>(buf: &mut B, value: u32) -> usize {
    if value < 0x80 {
        // Single byte: 0x00-0x7F (values 0-127)
        buf.put_u8(value as u8);
        1
    } else if value < 0x4080 {
        // Two bytes: 0x80-0xBF prefix + 8 bits
        // Value range: 128 to 16,511
        // Encoding: first byte = 0x80 | ((value-128) >> 8), second byte = (value-128) & 0xFF
        let adjusted = value - 0x80;
        buf.put_u8(0x80 | ((adjusted >> 8) as u8 & 0x3F));
        buf.put_u8(adjusted as u8);
        2
    } else {
        // Three bytes: 0xC0-0xFF prefix + 16 bits
        // Value range: 16,512 to 4,210,687
        let adjusted = value - 0x4080;
        buf.put_u8(0xC0 | ((adjusted >> 16) as u8 & 0x3F));
        buf.put_u8((adjusted >> 8) as u8);
        buf.put_u8(adjusted as u8);
        3
    }
}

/// Read a varint-encoded value from a byte slice.
///
/// Returns (value, bytes_consumed) or None if the buffer is too short.
pub fn read_varint(data: &[u8]) -> Option<(u32, usize)> {
    if data.is_empty() {
        return None;
    }

    let first = data[0];

    if first < 0x80 {
        // Single byte: 0x00-0x7F
        Some((first as u32, 1))
    } else if first < 0xC0 {
        // Two bytes: 0x80-0xBF prefix
        if data.len() < 2 {
            return None;
        }
        let high = (first & 0x3F) as u32;
        let low = data[1] as u32;
        Some((0x80 + (high << 8) + low, 2))
    } else {
        // Three bytes: 0xC0-0xFF prefix
        if data.len() < 3 {
            return None;
        }
        let high = (first & 0x3F) as u32;
        let mid = data[1] as u32;
        let low = data[2] as u32;
        Some((0x4080 + (high << 16) + (mid << 8) + low, 3))
    }
}

/// Maximum value that can be encoded in a varint.
pub const VARINT_MAX: u32 = 0x4080 + 0x3FFFFF; // 4,210,687

/// Check if a value can be encoded as a single byte.
pub fn is_single_byte(value: u32) -> bool {
    value < 0x80
}

/// Generate JavaScript code for decoding varints.
pub const VARINT_JS: &str = r#"function rv(d,i){let b=d[i];if(b<0x80)return[b,1];if(b<0xC0)return[0x80+((b&0x3F)<<8)+d[i+1],2];return[0x4080+((b&0x3F)<<16)+(d[i+1]<<8)+d[i+2],3]}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    fn encode(value: u32) -> Vec<u8> {
        let mut buf = BytesMut::new();
        write_varint(&mut buf, value);
        buf.to_vec()
    }

    fn roundtrip(value: u32) -> u32 {
        let encoded = encode(value);
        let (decoded, _) = read_varint(&encoded).unwrap();
        decoded
    }

    #[test]
    fn test_single_byte() {
        // Values 0-127 should encode as single byte
        assert_eq!(encode(0), vec![0x00]);
        assert_eq!(encode(1), vec![0x01]);
        assert_eq!(encode(127), vec![0x7F]);

        // Roundtrip
        for i in 0..128 {
            assert_eq!(roundtrip(i), i);
        }
    }

    #[test]
    fn test_two_bytes() {
        // Values 128-16,511 should encode as two bytes
        let encoded = encode(128);
        assert_eq!(encoded.len(), 2);
        assert!(encoded[0] >= 0x80 && encoded[0] < 0xC0);

        let encoded = encode(16511);
        assert_eq!(encoded.len(), 2);

        // Roundtrip
        assert_eq!(roundtrip(128), 128);
        assert_eq!(roundtrip(255), 255);
        assert_eq!(roundtrip(1000), 1000);
        assert_eq!(roundtrip(16511), 16511);
    }

    #[test]
    fn test_three_bytes() {
        // Values 16,512+ should encode as three bytes
        let encoded = encode(16512);
        assert_eq!(encoded.len(), 3);
        assert!(encoded[0] >= 0xC0);

        // Roundtrip
        assert_eq!(roundtrip(16512), 16512);
        assert_eq!(roundtrip(100000), 100000);
        assert_eq!(roundtrip(1000000), 1000000);
    }

    #[test]
    fn test_read_varint_insufficient_data() {
        // Single byte is always OK
        assert!(read_varint(&[0x50]).is_some());

        // Two-byte marker but only one byte
        assert!(read_varint(&[0x80]).is_none());

        // Three-byte marker but only two bytes
        assert!(read_varint(&[0xC0, 0x00]).is_none());

        // Empty buffer
        assert!(read_varint(&[]).is_none());
    }

    #[test]
    fn test_is_single_byte() {
        assert!(is_single_byte(0));
        assert!(is_single_byte(127));
        assert!(!is_single_byte(128));
        assert!(!is_single_byte(255));
    }

    #[test]
    fn test_boundary_values() {
        // Test the exact boundary values
        assert_eq!(encode(0x7F).len(), 1);  // 127 - max single byte
        assert_eq!(encode(0x80).len(), 2);  // 128 - min two bytes
        assert_eq!(encode(0x407F).len(), 2); // 16,511 - max two bytes
        assert_eq!(encode(0x4080).len(), 3); // 16,512 - min three bytes
    }
}
