//! Tests for the binary protocol encoder and decoder.

use rwire::protocol::*;

mod encoder {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buf = OpcodeBuffer::new();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_create_element() {
        let mut buf = OpcodeBuffer::new();
        let ref0 = buf.create(El::Div.as_u8());
        let ref1 = buf.create(El::Button.as_u8());

        assert_eq!(ref0, 0);
        assert_eq!(ref1, 1);

        let bytes = buf.finish();
        assert_eq!(
            bytes.as_ref(),
            &[0x02, El::Div.as_u8(), 0x02, El::Button.as_u8()]
        );
    }

    #[test]
    fn test_symbols() {
        let mut buf = OpcodeBuffer::new();
        buf.begin_symbols(2);
        let sym0 = buf.add_symbol("hello");
        let sym1 = buf.add_symbol("world");

        assert_eq!(sym0, 0x80);
        assert_eq!(sym1, 0x81);

        let bytes = buf.finish();
        assert_eq!(bytes[0], 0xF0); // SYMBOLS
        assert_eq!(bytes[1], 2); // count
        assert_eq!(bytes[2], 5); // len "hello"
        assert_eq!(&bytes[3..8], b"hello");
        assert_eq!(bytes[8], 5); // len "world"
        assert_eq!(&bytes[9..14], b"world");
    }

    #[test]
    fn test_set_operations() {
        let mut buf = OpcodeBuffer::new();
        let el = buf.create(El::Div.as_u8());
        buf.set_class(el, 0x80);
        buf.set_text(el, 0x81);
        buf.set_attr(el, 0x82, 0x83);
        buf.set_data(el, 0x84, 0x85);

        let bytes = buf.finish();
        assert_eq!(
            bytes.as_ref(),
            &[
                0x02, 0x00, // CREATE div
                0x10, 0, 0x80, // SET_CLASS
                0x11, 0, 0x81, // SET_TEXT
                0x12, 0, 0x82, 0x83, // SET_ATTR
                0x14, 0, 0x84, 0x85, // SET_DATA
            ]
        );
    }

    #[test]
    fn test_append() {
        let mut buf = OpcodeBuffer::new();
        let parent = buf.create(El::Div.as_u8());
        let child = buf.create(El::Span.as_u8());
        buf.append(parent, child);

        let bytes = buf.finish();
        assert_eq!(bytes.as_ref(), &[0x02, 0x00, 0x02, 0x01, 0x20, 0, 1]);
    }

    #[test]
    fn test_append_to_body() {
        let mut buf = OpcodeBuffer::new();
        let el = buf.create(El::Div.as_u8());
        buf.append_to_body(el);

        let bytes = buf.finish();
        assert_eq!(bytes.as_ref(), &[0x02, 0x00, 0x20, 0xFF, 0]);
    }

    #[test]
    fn test_bind_handlers() {
        let mut buf = OpcodeBuffer::new();
        let el = buf.create(El::Button.as_u8());
        buf.bind_local(el, Ev::Click.as_u8(), 0);
        buf.bind_remote(el, Ev::Submit.as_u8(), 1);
        buf.bind_optimistic(el, Ev::Input.as_u8(), 2);

        let bytes = buf.finish();
        assert_eq!(
            bytes.as_ref(),
            &[
                0x02,
                El::Button.as_u8(),
                0x30,
                0,
                Ev::Click.as_u8(),
                0,
                0x31,
                0,
                Ev::Submit.as_u8(),
                1,
                0x32,
                0,
                Ev::Input.as_u8(),
                2,
            ]
        );
    }

    #[test]
    fn test_get_by_id() {
        let mut buf = OpcodeBuffer::new();
        let ref0 = buf.get_by_id(0x80);
        let ref1 = buf.get_by_id(0x81);

        assert_eq!(ref0, 0);
        assert_eq!(ref1, 1);

        let bytes = buf.finish();
        assert_eq!(bytes.as_ref(), &[0x01, 0x80, 0x01, 0x81]);
    }

    #[test]
    fn test_end() {
        let mut buf = OpcodeBuffer::new();
        buf.create(El::Div.as_u8());
        buf.end();

        let bytes = buf.finish();
        assert_eq!(bytes.as_ref(), &[0x02, 0x00, 0xFF]);
    }

    #[test]
    fn test_full_message() {
        let mut buf = OpcodeBuffer::new();
        buf.begin_symbols(1);
        buf.add_symbol("btn");
        let el = buf.create(El::Button.as_u8());
        buf.set_class(el, 0x80);
        buf.bind_local(el, Ev::Click.as_u8(), 0);
        buf.append_to_body(el);
        buf.end();

        let bytes = buf.finish();
        assert_eq!(
            bytes.as_ref(),
            &[
                0xF0,
                1,
                3,
                b'b',
                b't',
                b'n', // SYMBOLS
                0x02,
                El::Button.as_u8(), // CREATE
                0x10,
                0,
                0x80, // SET_CLASS
                0x30,
                0,
                Ev::Click.as_u8(),
                0, // BIND_LOCAL
                0x20,
                0xFF,
                0,    // APPEND to body
                0xFF, // END
            ]
        );
    }

    #[test]
    fn test_default() {
        let buf = OpcodeBuffer::default();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_create_synced() {
        use rwire::protocol::opcodes::CREATE_SYNCED;

        let mut buf = OpcodeBuffer::new();
        let ref0 = buf.create_synced(0);
        let ref1 = buf.create_synced(1);
        let ref2 = buf.create_synced(127);

        assert_eq!(ref0, 0);
        assert_eq!(ref1, 1);
        assert_eq!(ref2, 2);

        let bytes = buf.finish();
        // Each CREATE_SYNCED has [opcode, varint_id]
        // For IDs < 128, varint is 1 byte
        assert_eq!(
            bytes.as_ref(),
            &[
                CREATE_SYNCED,
                0, // create __synced_0
                CREATE_SYNCED,
                1, // create __synced_1
                CREATE_SYNCED,
                127, // create __synced_127
            ]
        );
    }

    #[test]
    fn test_create_synced_large_ids() {
        use rwire::protocol::opcodes::CREATE_SYNCED;

        let mut buf = OpcodeBuffer::new();
        let ref0 = buf.create_synced(128); // First 2-byte varint
        let ref1 = buf.create_synced(1000);

        assert_eq!(ref0, 0);
        assert_eq!(ref1, 1);

        let bytes = buf.finish();
        // Verify opcode is present for both
        assert_eq!(bytes[0], CREATE_SYNCED);
        // ID 128 encodes as 2 bytes: 0x80 | ((128-128)>>8) = 0x80, (128-128)&0xFF = 0x00
        assert_eq!(bytes[1], 0x80);
        assert_eq!(bytes[2], 0x00);
    }

    #[test]
    fn test_get_synced() {
        use rwire::protocol::opcodes::GET_SYNCED;

        let mut buf = OpcodeBuffer::new();
        let ref0 = buf.get_synced(0);
        let ref1 = buf.get_synced(5);
        let ref2 = buf.get_synced(42);

        assert_eq!(ref0, 0);
        assert_eq!(ref1, 1);
        assert_eq!(ref2, 2);

        let bytes = buf.finish();
        assert_eq!(
            bytes.as_ref(),
            &[
                GET_SYNCED, 0, // get __synced_0
                GET_SYNCED, 5, // get __synced_5
                GET_SYNCED, 42, // get __synced_42
            ]
        );
    }

    #[test]
    fn test_symbols_extend() {
        use rwire::protocol::opcodes::SYMBOLS_EXTEND;

        let mut buf = OpcodeBuffer::new();
        // Simulate having already sent 2 symbols (indices 0x80 and 0x81)
        buf.begin_symbols_extend(2, 0x82);
        let sym0 = buf.add_symbol("new1");
        let sym1 = buf.add_symbol("new2");

        // Should start at 0x82 (after the 2 existing)
        assert_eq!(sym0, 0x82);
        assert_eq!(sym1, 0x83);

        let bytes = buf.finish();
        assert_eq!(bytes[0], SYMBOLS_EXTEND);
        assert_eq!(bytes[1], 2); // count
        assert_eq!(bytes[2], 4); // len "new1"
        assert_eq!(&bytes[3..7], b"new1");
        assert_eq!(bytes[7], 4); // len "new2"
        assert_eq!(&bytes[8..12], b"new2");
    }

    #[test]
    fn test_synced_opcodes_byte_savings() {
        // Verify byte savings from using CREATE_SYNCED/GET_SYNCED
        use rwire::protocol::opcodes::{CREATE_SYNCED, GET_SYNCED};

        // Old approach: CREATE span (2 bytes) + SET_ATTR id (4 bytes) + symbol table overhead (~15 bytes)
        // = ~21 bytes per synced element

        // New approach: CREATE_SYNCED (2 bytes for id < 128)
        // = 2 bytes per synced element

        // Test CREATE_SYNCED with small ID
        let mut buf = OpcodeBuffer::new();
        buf.create_synced(0);
        let bytes = buf.finish();
        assert_eq!(bytes.len(), 2); // opcode + varint(0)
        assert_eq!(bytes[0], CREATE_SYNCED);
        assert_eq!(bytes[1], 0);

        // Test GET_SYNCED with small ID
        let mut buf = OpcodeBuffer::new();
        buf.get_synced(5);
        let bytes = buf.finish();
        assert_eq!(bytes.len(), 2); // opcode + varint(5)
        assert_eq!(bytes[0], GET_SYNCED);
        assert_eq!(bytes[1], 5);

        // Savings: ~19 bytes per synced element reference!
    }

    #[test]
    fn test_word_table() {
        use rwire::protocol::opcodes::WORD_TABLE;

        let mut buf = OpcodeBuffer::new();
        buf.begin_word_table(2);
        buf.add_word("hello");
        buf.add_word("world");

        let bytes = buf.finish();
        assert_eq!(bytes[0], WORD_TABLE);
        assert_eq!(bytes[1], 2); // count
        assert_eq!(bytes[2], 5); // len "hello"
        assert_eq!(&bytes[3..8], b"hello");
        assert_eq!(bytes[8], 5); // len "world"
        assert_eq!(&bytes[9..14], b"world");
    }

    #[test]
    fn test_set_text_words() {
        use rwire::protocol::opcodes::SET_TEXT_WORDS;

        let mut buf = OpcodeBuffer::new();
        let el = buf.create(El::Span.as_u8());
        buf.set_text_words(el, &[0, 1, 2]);

        let bytes = buf.finish();
        // CREATE Span (2 bytes) + SET_TEXT_WORDS (1 + 1 + 1 + 3 = 6 bytes)
        assert_eq!(bytes.len(), 8);
        assert_eq!(bytes[2], SET_TEXT_WORDS);
        assert_eq!(bytes[3], 0); // ref
        assert_eq!(bytes[4], 3); // count
        assert_eq!(bytes[5], 0); // word index 0
        assert_eq!(bytes[6], 1); // word index 1
        assert_eq!(bytes[7], 2); // word index 2
    }

    #[test]
    fn test_set_text_int() {
        use rwire::protocol::opcodes::SET_TEXT_INT;

        // Test positive number
        let mut buf = OpcodeBuffer::new();
        let el = buf.create(El::Span.as_u8());
        buf.set_text_int(el, 42);

        let bytes = buf.finish();
        assert_eq!(bytes[2], SET_TEXT_INT);
        assert_eq!(bytes[3], 0); // ref
        // 42 as zigzag: (42 << 1) ^ (42 >> 31) = 84
        assert_eq!(bytes[4], 84); // varint for 84 is single byte

        // Test negative number
        let mut buf = OpcodeBuffer::new();
        let el = buf.create(El::Span.as_u8());
        buf.set_text_int(el, -1);

        let bytes = buf.finish();
        // -1 as zigzag: (-1 << 1) ^ (-1 >> 31) = -2 ^ -1 = 1
        assert_eq!(bytes[4], 1);

        // Test zero
        let mut buf = OpcodeBuffer::new();
        let el = buf.create(El::Span.as_u8());
        buf.set_text_int(el, 0);

        let bytes = buf.finish();
        assert_eq!(bytes[4], 0);
    }

    #[test]
    fn test_text_compression_byte_savings() {
        // Compare byte costs of different text encoding approaches
        // This test documents byte savings from text compression opcodes

        // Scenario: Displaying "Hello world" using repeated words

        // Traditional approach: Symbol table + SET_TEXT
        // [SYMBOLS, 1, 11, "Hello world"] = 14 bytes
        // [SET_TEXT, ref, sym] = 3 bytes
        // Total: 17 bytes

        // Word table approach: WORD_TABLE + SET_TEXT_WORDS
        // [WORD_TABLE, 2, 5, "Hello", 5, "world"] = 14 bytes
        // [SET_TEXT_WORDS, ref, 2, 0, 1] = 5 bytes
        // Total: 19 bytes (slightly worse for single use)

        // BUT if the same words appear multiple times:
        // 10 uses of "Hello world" with symbol table:
        // 14 + 10*3 = 44 bytes

        // 10 uses of "Hello" and "world" separately:
        // 14 (word table) + 10*5 = 64 bytes
        // vs 14+14 (2 symbols) + 10*3*2 = 88 bytes

        // For integer values:
        // Symbol "123456789": 9 + 3 = 12 bytes
        // SET_TEXT_INT: 3 + 5 (varint) = 8 bytes max
        let mut buf = OpcodeBuffer::new();
        buf.begin_symbols(1);
        buf.add_symbol("123456789");
        let symbol_bytes = buf.finish();

        let mut buf = OpcodeBuffer::new();
        let el = buf.create(El::Span.as_u8());
        buf.set_text_int(el, 123456789);
        let int_bytes = buf.finish();

        // Symbol approach: SYMBOLS header (2) + string (10) = 12 bytes for table
        // Int approach: CREATE (2) + SET_TEXT_INT (1 + 1 + 4) = 8 bytes total
        println!("Symbol table for '123456789': {} bytes", symbol_bytes.len());
        println!("SET_TEXT_INT for 123456789: {} bytes", int_bytes.len());

        // Verify SET_TEXT_INT is more compact for numbers
        assert!(int_bytes.len() < symbol_bytes.len() + 3); // + 3 for SET_TEXT
    }
}

mod decoder {
    use super::*;

    #[test]
    fn test_decode_minimal_event() {
        let data = [0, Ev::Click.as_u8(), 1, 0];
        let event = ClientEvent::decode(&data).unwrap();

        assert_eq!(event.handler_idx, 0);
        assert_eq!(event.event_type, Ev::Click.as_u8());
        assert_eq!(event.target_ref, 1);
        assert!(event.payload.is_empty());
    }

    #[test]
    fn test_decode_with_payload() {
        let data = [5, Ev::Input.as_u8(), 2, 3, b'a', b'b', b'c'];
        let event = ClientEvent::decode(&data).unwrap();

        assert_eq!(event.handler_idx, 5);
        assert_eq!(event.event_type, Ev::Input.as_u8());
        assert_eq!(event.target_ref, 2);
        assert_eq!(event.payload, b"abc");
    }

    #[test]
    fn test_decode_too_short() {
        let data = [0, 1, 2];
        let result = ClientEvent::decode(&data);
        assert!(matches!(result, Err(DecodeError::TooShort)));
    }

    #[test]
    fn test_decode_payload_truncated() {
        let data = [0, Ev::Click.as_u8(), 0, 5, b'a', b'b'];
        let result = ClientEvent::decode(&data);
        assert!(matches!(result, Err(DecodeError::PayloadTruncated)));
    }

    #[test]
    fn test_decode_empty() {
        let result = ClientEvent::decode(&[]);
        assert!(matches!(result, Err(DecodeError::TooShort)));
    }

    #[test]
    fn test_event_type_names() {
        let events = [
            (Ev::Click, "click"),
            (Ev::DblClick, "dblclick"),
            (Ev::MouseDown, "mousedown"),
            (Ev::MouseUp, "mouseup"),
            (Ev::MouseMove, "mousemove"),
            (Ev::Submit, "submit"),
            (Ev::Input, "input"),
            (Ev::Change, "change"),
            (Ev::KeyDown, "keydown"),
            (Ev::KeyUp, "keyup"),
            (Ev::Focus, "focus"),
            (Ev::Blur, "blur"),
        ];

        for (ev, expected_name) in events {
            let event = ClientEvent {
                handler_idx: 0,
                event_type: ev.as_u8(),
                target_ref: 0,
                payload: vec![],
                param_bytes: vec![],
            };
            assert_eq!(event.event_type_name(), expected_name);
        }
    }

    #[test]
    fn test_unknown_event_type() {
        let event = ClientEvent {
            handler_idx: 0,
            event_type: 0xFF,
            target_ref: 0,
            payload: vec![],
            param_bytes: vec![],
        };
        assert_eq!(event.event_type_name(), "unknown");
    }

    #[test]
    fn test_decode_error_display() {
        assert_eq!(DecodeError::TooShort.to_string(), "message too short");
        assert_eq!(
            DecodeError::PayloadTruncated.to_string(),
            "payload truncated"
        );
    }
}

mod opcodes {
    use super::*;

    #[test]
    fn test_element_type_codes() {
        assert_eq!(El::Div.as_u8(), 0x00);
        assert_eq!(El::Span.as_u8(), 0x01);
        assert_eq!(El::Button.as_u8(), 0x02);
        assert_eq!(El::Input.as_u8(), 0x03);
        assert_eq!(El::P.as_u8(), 0x04);
        assert_eq!(El::H1.as_u8(), 0x05);
        assert_eq!(El::H2.as_u8(), 0x06);
        assert_eq!(El::A.as_u8(), 0x07);
        assert_eq!(El::Form.as_u8(), 0x10);
    }

    #[test]
    fn test_element_codes_unique() {
        let codes: Vec<u8> = vec![
            El::Div.as_u8(),
            El::Span.as_u8(),
            El::Button.as_u8(),
            El::Input.as_u8(),
            El::Form.as_u8(),
            El::P.as_u8(),
            El::H1.as_u8(),
            El::H2.as_u8(),
            El::A.as_u8(),
        ];
        let mut sorted = codes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(codes.len(), sorted.len(), "Element codes must be unique");
    }

    #[test]
    fn test_event_type_codes() {
        assert_eq!(Ev::Click.as_u8(), 0x01);
        assert_eq!(Ev::DblClick.as_u8(), 0x02);
        assert_eq!(Ev::MouseDown.as_u8(), 0x03);
        assert_eq!(Ev::MouseUp.as_u8(), 0x04);
        assert_eq!(Ev::MouseMove.as_u8(), 0x05);
        assert_eq!(Ev::Submit.as_u8(), 0x06);
        assert_eq!(Ev::Input.as_u8(), 0x07);
        assert_eq!(Ev::Change.as_u8(), 0x08);
        assert_eq!(Ev::KeyDown.as_u8(), 0x09);
        assert_eq!(Ev::KeyUp.as_u8(), 0x0A);
        assert_eq!(Ev::Focus.as_u8(), 0x0B);
        assert_eq!(Ev::Blur.as_u8(), 0x0C);
    }

    #[test]
    fn test_event_codes_unique() {
        let codes: Vec<u8> = vec![
            Ev::Click.as_u8(),
            Ev::DblClick.as_u8(),
            Ev::MouseDown.as_u8(),
            Ev::MouseUp.as_u8(),
            Ev::MouseMove.as_u8(),
            Ev::Submit.as_u8(),
            Ev::Input.as_u8(),
            Ev::Change.as_u8(),
            Ev::KeyDown.as_u8(),
            Ev::KeyUp.as_u8(),
            Ev::Focus.as_u8(),
            Ev::Blur.as_u8(),
        ];
        let mut sorted = codes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(codes.len(), sorted.len(), "Event codes must be unique");
    }
}
