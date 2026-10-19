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

/// Largest payload one frame may carry. A bigger change is split into several frames.
pub const MAX_PAYLOAD: usize = 1024 * 1024;

/// Why a binary frame was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
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
    /// `len` exceeds [`MAX_PAYLOAD`].
    TooLarge,
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
    /// Split one changed run into frames of at most [`MAX_PAYLOAD`] bytes. All share `seq`.
    pub fn split(file_id: u32, seq: u64, offset: u64, data: &[u8]) -> Vec<RangeFrame> {
        data.chunks(MAX_PAYLOAD)
            .enumerate()
            .map(|(i, chunk)| RangeFrame {
                file_id,
                seq,
                offset: offset + (i * MAX_PAYLOAD) as u64,
                data: chunk.to_vec(),
            })
            .collect()
    }

    /// Encode to wire bytes. `data` must be 1 to [`MAX_PAYLOAD`] bytes; use [`RangeFrame::split`].
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
        if len as usize > MAX_PAYLOAD {
            return Err(FrameError::TooLarge);
        }
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
#[non_exhaustive]
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

/// Client to server. Carries the protocol version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authenticate {
    pub request_id: String,
    pub protocol_version: u32,
    pub tenant_id: String,
    pub client_id: String,
    pub auth_method: String,
    pub credentials: String,
}

/// Server to client: the flush identified by `request_id` is applied; `seq` is the new sequence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlushAck {
    pub request_id: String,
    pub file_id: u32,
    pub seq: u64,
}

/// Client to server: send the whole file again (no replay log in v1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resync {
    pub request_id: String,
    pub file_id: u32,
}

/// Server to client: the last range frame of the snapshot has been sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResyncDone {
    pub request_id: String,
    pub file_id: u32,
    pub seq: u64,
    pub size: u64,
}

/// Server to client: a request failed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorReply {
    pub code: ErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Client to server: may this service file be mapped locally?
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetServiceFilePath {
    pub request_id: String,
    pub service_id: String,
}

/// Server to client: the `file_id` that range frames use for this service. `path` and `nonce_path`
/// are both present (local mapping offered) or both absent (stay on WSS ranges). They are absent
/// unless the service is open to local mapping and the caller holds a write grant on it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceFilePath {
    pub request_id: String,
    pub service_id: String,
    pub file_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce_path: Option<String>,
}

/// Client to server: the nonce read from `nonce_path`, proving the same filesystem view.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmLocal {
    pub request_id: String,
    pub file_id: u32,
    pub nonce: String,
}

impl std::fmt::Debug for ConfirmLocal {
    // The nonce is a secret for the life of the connection; never print it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfirmLocal")
            .field("request_id", &self.request_id)
            .field("file_id", &self.file_id)
            .field("nonce", &"<redacted>")
            .finish()
    }
}

/// Server to client: the nonce matched; the server deletes the nonce file. A mismatch is an
/// `Error` with `PERMISSION_DENIED`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmLocalAck {
    pub request_id: String,
    pub file_id: u32,
}

/// Why a service file is being replaced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum RotateReason {
    /// The file changes size. A drain barrier runs first; old-offset writes are never replayed.
    Resize,
    /// A client is removed. Same size and shape; old bytes are kept; no drain barrier.
    Revocation,
}

/// Client to server: replace a service's file. `new_size` is required for `RESIZE` and forbidden
/// for `REVOCATION`; `revoke_client_id` is the reverse. See [`RotateRequest::validate`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotateRequest {
    pub request_id: String,
    pub service_id: String,
    pub reason: RotateReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoke_client_id: Option<String>,
}

impl RotateRequest {
    /// Check the fields agree with `reason`. A request that fails is answered `INVALID_REQUEST`.
    pub fn validate(&self) -> Result<(), &'static str> {
        match self.reason {
            RotateReason::Resize => match (self.new_size, &self.revoke_client_id) {
                (Some(n), None) if n > 0 => Ok(()),
                (Some(0), _) => Err("new_size must be at least 1"),
                (None, _) => Err("RESIZE needs new_size"),
                (_, Some(_)) => Err("RESIZE must not name a client"),
                _ => Err("invalid RESIZE request"),
            },
            RotateReason::Revocation => match (self.new_size, &self.revoke_client_id) {
                (None, Some(c)) if !c.is_empty() => Ok(()),
                (_, None) => Err("REVOCATION needs revoke_client_id"),
                (Some(_), _) => Err("REVOCATION must not set new_size"),
                _ => Err("invalid REVOCATION request"),
            },
        }
    }
}

/// Server to client: the request was accepted. The rotation ends with [`RotationCommit`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationAccepted {
    pub request_id: String,
    pub rotation_id: String,
}

/// Server to every SDK on the service (resize only): the drain barrier is open. Stop issuing write
/// guards, wait for outstanding ones, flush into the old file, then send [`RotationDrained`] within
/// `deadline_ms`. An SDK that does not is dropped with a `ROTATION_STRAGGLER` [`Loss`]. After the
/// barrier the requester fills the new file; the rotation ends with a [`RotationCommit`] or a
/// [`RotationAborted`] within `fill_ms` more (see `spec/rotation.md`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationPending {
    pub rotation_id: String,
    pub service_id: String,
    pub file_id: u32,
    pub deadline_ms: u64,
    pub fill_ms: u64,
}

/// Server to the requester only, after the drain barrier (resize only): the new file exists at
/// `new_file_id`, zero filled, and is not live yet. Fill it from the old file in the new shape, then
/// send [`RotationFilled`]. A local requester gets `path` and writes it directly; a remote requester
/// has no `path` and sends range frames for `new_file_id`, ending with `FlushEnd`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationPrepared {
    pub rotation_id: String,
    pub service_id: String,
    pub old_file_id: u32,
    pub new_file_id: u32,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Requester to server: the new file is filled; commit the rotation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationFilled {
    pub request_id: String,
    pub rotation_id: String,
}

/// Requester to server: give up; keep the old file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationAbort {
    pub request_id: String,
    pub rotation_id: String,
}

/// Why a rotation was abandoned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum AbortReason {
    /// The requester sent `RotationAbort`.
    RequesterAbort,
    /// The requester did not send `RotationFilled` within `fill_ms`.
    FillTimeout,
    /// The server could not build or switch to the new file.
    ServerError,
    /// A reason this version does not know. Treat the rotation as abandoned.
    #[serde(other)]
    Unknown,
}

/// Server to every member (and the requester): the rotation is abandoned. The old file stays live;
/// writers resume on it. Nothing was replayed or lost.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationAborted {
    pub rotation_id: String,
    pub service_id: String,
    pub file_id: u32,
    pub reason: AbortReason,
}

/// Client to server: everything written so far is flushed; `seq` is the last sequence flushed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationDrained {
    pub request_id: String,
    pub rotation_id: String,
    pub file_id: u32,
    pub seq: u64,
}

/// Server to every SDK on the service: the new file is live. `new_file_id` differs from
/// `old_file_id` and is the handle for the new mapping; fetch it with `GetServiceFilePath` (local)
/// or `Resync` (remote). After a revocation, each remaining SDK writes its unflushed ranges into
/// the new file; after a resize, old-offset writes are never replayed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationCommit {
    pub rotation_id: String,
    pub service_id: String,
    pub reason: RotateReason,
    pub old_file_id: u32,
    pub new_file_id: u32,
    pub generation: u64,
    pub size: u64,
}

/// What a program lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum LossKind {
    /// It did not drain in time (or kept writing to the old mapping); those writes are not replayed.
    RotationStraggler,
    /// A resync overwrote local changes; `ranges` names them.
    ResyncOverwrote,
    /// Its access was revoked; unflushed writes are dropped.
    Revoked,
    /// A kind this version does not know. Treat it as a loss of the whole file.
    #[serde(other)]
    Unknown,
}

/// A byte range inside a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LossRange {
    pub offset: u64,
    pub len: u64,
}

/// Server to a client: writes of its own were lost, and why. `ranges` are in old-file coordinates;
/// empty means the whole file. A receiver treats an unknown `kind` as a loss of the whole file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Loss {
    pub service_id: String,
    pub file_id: u32,
    pub kind: LossKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ranges: Vec<LossRange>,
}

/// A run of bytes inside a file, named without its data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByteRange {
    pub offset: u64,
    pub len: u64,
}

/// Most ranges one `RangesChanged` or `RangesUpdated` may carry. A flush with more is split into
/// several messages, each getting its own `seq`.
pub const MAX_RANGES_PER_MESSAGE: usize = 1024;

/// Check a range list: 1 to [`MAX_RANGES_PER_MESSAGE`] ranges, none empty, none whose end overflows a
/// u64, sorted by offset and not overlapping (adjacent is fine). A message that fails is answered
/// `INVALID_REQUEST` (or dropped, when it came from the server).
pub fn validate_ranges(ranges: &[ByteRange]) -> Result<(), &'static str> {
    if ranges.is_empty() {
        return Err("ranges must not be empty");
    }
    if ranges.len() > MAX_RANGES_PER_MESSAGE {
        return Err("too many ranges in one message");
    }
    let mut prev_end = 0u64;
    for r in ranges {
        if r.len == 0 {
            return Err("a range must cover at least one byte");
        }
        let Some(end) = r.offset.checked_add(r.len) else {
            return Err("offset plus len overflows");
        };
        if r.offset < prev_end {
            return Err("ranges must be sorted and must not overlap");
        }
        prev_end = end;
    }
    Ok(())
}

/// Client to server, from a **local** sender (it wrote into the shared mapping): these ranges
/// changed. No data travels; the server reads the latest bytes from the file when it forwards.
/// Answered with `FlushAck`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangesChanged {
    pub request_id: String,
    pub file_id: u32,
    pub ranges: Vec<ByteRange>,
}

/// Server to a **local** receiver: these ranges of the file changed, at `seq`. No data; the receiver
/// already sees the bytes through its own mapping.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangesUpdated {
    pub file_id: u32,
    pub seq: u64,
    pub ranges: Vec<ByteRange>,
}

/// Client to server, from a **remote** sender: the range frames for one flush are complete. The
/// server assigns the flush one new `seq`, forwards it, and answers `FlushAck`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlushEnd {
    pub request_id: String,
    pub file_id: u32,
}

/// JSON control messages, sent as WebSocket text frames. Adjacently tagged
/// (`{"type": ..., "data": {...}}`); unknown fields and unknown types are refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[non_exhaustive]
pub enum ControlMessage {
    Authenticate(Authenticate),
    FlushAck(FlushAck),
    Resync(Resync),
    ResyncDone(ResyncDone),
    Error(ErrorReply),
    GetServiceFilePath(GetServiceFilePath),
    ServiceFilePath(ServiceFilePath),
    ConfirmLocal(ConfirmLocal),
    ConfirmLocalAck(ConfirmLocalAck),
    RotateRequest(RotateRequest),
    RotationAccepted(RotationAccepted),
    RotationPending(RotationPending),
    RotationPrepared(RotationPrepared),
    RotationFilled(RotationFilled),
    RotationAbort(RotationAbort),
    RotationAborted(RotationAborted),
    RotationDrained(RotationDrained),
    RotationCommit(RotationCommit),
    Loss(Loss),
    RangesChanged(RangesChanged),
    RangesUpdated(RangesUpdated),
    FlushEnd(FlushEnd),
}
