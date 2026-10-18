//! Runs the language-neutral golden vectors in `vectors/` against the Rust types.
//! Every non-Rust SDK runs the same files in its own CI.

use commy_wire::{
    ControlMessage, ErrorCode, FrameError, RangeFrame, HEADER_LEN, KIND_RANGE, MAX_PAYLOAD,
};
use serde_json::Value;

fn load(name: &str) -> Value {
    let path = format!("{}/vectors/{}", env!("CARGO_MANIFEST_DIR"), name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap()
}

fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "odd hex length");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// The frame bytes of a vector: `hex`, or `header_hex` plus a repeated payload byte.
fn frame_bytes(c: &Value) -> Vec<u8> {
    if let Some(h) = c.get("hex") {
        return unhex(h.as_str().unwrap());
    }
    let mut out = unhex(c["header_hex"].as_str().unwrap());
    let r = &c["payload_repeat"];
    let byte = unhex(r["byte"].as_str().unwrap())[0];
    out.extend(std::iter::repeat_n(
        byte,
        r["count"].as_u64().unwrap() as usize,
    ));
    out
}

#[test]
fn max_payload_is_one_mebibyte() {
    assert_eq!(MAX_PAYLOAD, 1024 * 1024);
}

#[test]
fn header_is_25_bytes() {
    assert_eq!(HEADER_LEN, 25);
    assert_eq!(KIND_RANGE, 1);
}

#[test]
fn valid_range_frames_decode_and_reencode_identically() {
    let v = load("range_frame.json");
    let cases = v["valid"].as_array().unwrap();
    assert!(!cases.is_empty());
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let bytes = frame_bytes(c);
        let frame = RangeFrame::decode(&bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(
            frame.file_id as u64,
            c["file_id"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(frame.seq, c["seq"].as_u64().unwrap(), "{name}");
        assert_eq!(frame.offset, c["offset"].as_u64().unwrap(), "{name}");
        if let Some(d) = c.get("data_hex") {
            assert_eq!(frame.data, unhex(d.as_str().unwrap()), "{name}");
        } else {
            assert_eq!(frame.data, bytes[HEADER_LEN..], "{name}");
        }
        assert_eq!(frame.encode(), bytes, "{name}: re-encode differs");
    }
}

#[test]
fn invalid_range_frames_are_refused_with_the_named_reason() {
    let v = load("range_frame.json");
    let cases = v["invalid"].as_array().unwrap();
    assert!(!cases.is_empty());
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let bytes = frame_bytes(c);
        let err = RangeFrame::decode(&bytes).expect_err(name);
        let want = match c["error"].as_str().unwrap() {
            "Truncated" => FrameError::Truncated,
            "BadKind" => FrameError::BadKind,
            "LengthMismatch" => FrameError::LengthMismatch,
            "EmptyRange" => FrameError::EmptyRange,
            "OffsetOverflow" => FrameError::OffsetOverflow,
            "TooLarge" => FrameError::TooLarge,
            other => panic!("unknown error name {other}"),
        };
        assert_eq!(err, want, "{name}");
    }
}

#[test]
fn error_codes_have_one_spelling() {
    let v = load("error_codes.json");
    let codes = v["codes"].as_array().unwrap();
    assert_eq!(codes.len(), ErrorCode::ALL.len());
    for (code, want) in ErrorCode::ALL.iter().zip(codes) {
        let want = want.as_str().unwrap();
        assert_eq!(code.as_str(), want);
        assert_eq!(serde_json::to_string(code).unwrap(), format!("\"{want}\""));
        let back: ErrorCode = serde_json::from_str(&format!("\"{want}\"")).unwrap();
        assert_eq!(&back, code);
    }
}

#[test]
fn legacy_spellings_are_not_accepted() {
    for legacy in [
        "InvalidRequest",
        "Unauthorized",
        "NotFound",
        "InternalError",
        "unauthorized",
    ] {
        assert!(
            serde_json::from_str::<ErrorCode>(&format!("\"{legacy}\"")).is_err(),
            "{legacy} must not parse"
        );
    }
}

#[test]
fn valid_control_messages_round_trip() {
    let v = load("control_messages.json");
    let cases = v["valid"].as_array().unwrap();
    assert!(!cases.is_empty());
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let msg: ControlMessage =
            serde_json::from_value(c["json"].clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(serde_json::to_value(&msg).unwrap(), c["json"], "{name}");
    }
}

#[test]
fn invalid_control_messages_are_refused() {
    let v = load("control_messages.json");
    let cases = v["invalid"].as_array().unwrap();
    assert!(!cases.is_empty());
    for c in cases {
        let name = c["name"].as_str().unwrap();
        assert!(
            serde_json::from_value::<ControlMessage>(c["json"].clone()).is_err(),
            "{name} must be refused"
        );
    }
}

#[test]
fn split_chunks_at_the_maximum_and_keeps_seq_and_offsets() {
    let data = vec![7u8; MAX_PAYLOAD * 2 + 5];
    let frames = RangeFrame::split(9, 4, 100, &data);
    assert_eq!(frames.len(), 3);
    assert!(frames.iter().all(|f| f.seq == 4 && f.file_id == 9));
    assert_eq!(frames[1].offset, 100 + MAX_PAYLOAD as u64);
    assert_eq!(frames[2].data.len(), 5);
    for f in &frames {
        assert_eq!(&RangeFrame::decode(&f.encode()).unwrap(), f);
    }
}

#[test]
fn the_nonce_never_appears_in_debug_output() {
    let m = commy_wire::ConfirmLocal {
        request_id: "r".into(),
        file_id: 1,
        nonce: "s3cr3t-nonce".into(),
    };
    assert!(!format!("{m:?}").contains("s3cr3t-nonce"));
}
