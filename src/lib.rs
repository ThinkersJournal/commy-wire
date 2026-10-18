//! The Commy wire contract: the binary range frame and the error-code vocabulary.
//!
//! The specification is in `spec/`, the language-neutral golden vectors are in
//! `vectors/`, and every SDK runs the same vectors in its own CI. This crate is
//! the Rust side of that contract and is tested against those exact files.

use serde::{Deserialize, Serialize};

/// First byte of a binary range frame.
pub const KIND_RANGE: u8 = 1;

/// Bytes before the payload: kind (1) + file_id (4) + seq (8) + offset (8) + len (4).
pub const HEADER_LEN: usize = 25;

/// Why a binary frame was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// Fewer bytes than the header needs.
    Truncated,
    /// The first byte is not a known frame kind.
    BadKind,
    /// The `len` field disagrees with the number of payload bytes.
    LengthMismatch,
    /// A range of zero bytes carries nothing and is refused.
    EmptyRange,
    /// `offset + len` does not fit in a u64.
    OffsetOverflow,
}

/// A run of bytes that changed in one service file. All integers are little-endian.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeFrame {
    pub file_id: u32,
    pub seq: u64,
    pub offset: u64,
    pub data: Vec<u8>,
}

impl RangeFrame {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.data.len());
        out.push(KIND_RANGE);
        out.extend_from_slice(&self.file_id.to_le_bytes());
        out.extend_from_slice(&self.seq.to_le_bytes());
        out.extend_from_slice(&self.offset.to_le_bytes());
        out.extend_from_slice(&(self.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.data);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<RangeFrame, FrameError> {
        if bytes.len() < HEADER_LEN {
            return Err(FrameError::Truncated);
        }
        if bytes[0] != KIND_RANGE {
            return Err(FrameError::BadKind);
        }
        let file_id = u32::from_le_bytes(bytes[1..5].try_into().unwrap());
        let seq = u64::from_le_bytes(bytes[5..13].try_into().unwrap());
        let offset = u64::from_le_bytes(bytes[13..21].try_into().unwrap());
        let len = u32::from_le_bytes(bytes[21..25].try_into().unwrap());
        let data = &bytes[HEADER_LEN..];
        if data.len() as u64 != len as u64 {
            return Err(FrameError::LengthMismatch);
        }
        if len == 0 {
            return Err(FrameError::EmptyRange);
        }
        if offset.checked_add(len as u64).is_none() {
            return Err(FrameError::OffsetOverflow);
        }
        Ok(RangeFrame {
            file_id,
            seq,
            offset,
            data: data.to_vec(),
        })
    }
}

/// The only spellings an error code may have on the wire, from server and SDKs alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidRequest,
    Unauthenticated,
    PermissionDenied,
    NotFound,
    RateLimited,
    UnsupportedVersion,
    InternalError,
}

impl ErrorCode {
    /// Every code, in the order `spec/errors.md` lists them.
    pub const ALL: [ErrorCode; 7] = [
        ErrorCode::InvalidRequest,
        ErrorCode::Unauthenticated,
        ErrorCode::PermissionDenied,
        ErrorCode::NotFound,
        ErrorCode::RateLimited,
        ErrorCode::UnsupportedVersion,
        ErrorCode::InternalError,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::InvalidRequest => "INVALID_REQUEST",
            ErrorCode::Unauthenticated => "UNAUTHENTICATED",
            ErrorCode::PermissionDenied => "PERMISSION_DENIED",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::RateLimited => "RATE_LIMITED",
            ErrorCode::UnsupportedVersion => "UNSUPPORTED_VERSION",
            ErrorCode::InternalError => "INTERNAL_ERROR",
        }
    }
}
