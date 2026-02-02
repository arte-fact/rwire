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
