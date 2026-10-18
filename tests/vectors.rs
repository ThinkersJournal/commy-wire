//! Runs the language-neutral golden vectors in `vectors/` against the Rust types.
//! Every non-Rust SDK runs the same files in its own CI.

use commy_wire::{ErrorCode, FrameError, RangeFrame, HEADER_LEN, KIND_RANGE};
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
        let bytes = unhex(c["hex"].as_str().unwrap());
        let frame = RangeFrame::decode(&bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(
            frame.file_id as u64,
            c["file_id"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(frame.seq, c["seq"].as_u64().unwrap(), "{name}");
        assert_eq!(frame.offset, c["offset"].as_u64().unwrap(), "{name}");
        assert_eq!(frame.data, unhex(c["data_hex"].as_str().unwrap()), "{name}");
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
        let bytes = unhex(c["hex"].as_str().unwrap());
        let err = RangeFrame::decode(&bytes).expect_err(name);
        let want = match c["error"].as_str().unwrap() {
            "Truncated" => FrameError::Truncated,
            "BadKind" => FrameError::BadKind,
            "LengthMismatch" => FrameError::LengthMismatch,
            "EmptyRange" => FrameError::EmptyRange,
            "OffsetOverflow" => FrameError::OffsetOverflow,
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
