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

/// The protocol version a client sends in [`Authenticate`] and this crate describes.
pub const PROTOCOL_VERSION: u32 = 1;

/// Longest `client_id` accepted.
pub const MAX_CLIENT_ID_LEN: usize = 128;

/// Longest `tenant_id` accepted.
pub const MAX_TENANT_ID_LEN: usize = 64;

/// How a client proves who it is. Tagged by `method`; the fields sit beside the tag in
/// [`Authenticate`]. Secrets are redacted from `Debug` output.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum Credentials {
    /// A bearer API key.
    #[serde(rename = "api_key")]
    ApiKey { key: String },
    /// A JSON Web Token.
    #[serde(rename = "jwt")]
    Jwt { token: String },
    /// A user name and password.
    #[serde(rename = "basic")]
    Basic { username: String, password: String },
    /// Anything a custom verifier on the server understands.
    #[serde(rename = "custom")]
    Custom { data: serde_json::Value },
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Credentials::ApiKey { .. } => f
                .debug_struct("ApiKey")
                .field("key", &"<redacted>")
                .finish(),
            Credentials::Jwt { .. } => f.debug_struct("Jwt").field("token", &"<redacted>").finish(),
            Credentials::Basic { username, .. } => f
                .debug_struct("Basic")
                .field("username", username)
                .field("password", &"<redacted>")
                .finish(),
            Credentials::Custom { .. } => f
                .debug_struct("Custom")
                .field("data", &"<redacted>")
                .finish(),
        }
    }
}

impl Credentials {
    /// The wire name of the method.
    pub fn method(&self) -> &'static str {
        match self {
            Credentials::ApiKey { .. } => "api_key",
            Credentials::Jwt { .. } => "jwt",
            Credentials::Basic { .. } => "basic",
            Credentials::Custom { .. } => "custom",
        }
    }
}

/// Client to server: authenticate to a tenant. The first message on a connection.
///
/// `client_id` is a **hint for development servers only**: a production server takes the client's
/// identity from the credential (its subject) and ignores or refuses a self-claimed one. `dev_scopes`
/// is likewise development-only: ask for exactly these scopes (`[]` for none) instead of administrator.
/// Unknown fields are ignored, because the credential fields are flattened next to the others.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authenticate {
    pub request_id: String,
    pub protocol_version: u32,
    pub tenant_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dev_scopes: Option<Vec<String>>,
    #[serde(flatten)]
    pub credentials: Credentials,
}

impl Authenticate {
    /// Check the fields. A request that fails is answered `INVALID_REQUEST`.
    pub fn validate(&self) -> Result<(), &'static str> {
        let t = &self.tenant_id;
        if t.is_empty()
            || t.len() > MAX_TENANT_ID_LEN
            || !t
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err("tenant_id must be 1 to 64 characters of letters, digits, _ and -");
        }
        if let Some(c) = &self.client_id {
            if c.is_empty()
                || c.len() > MAX_CLIENT_ID_LEN
                || !c.chars().all(|ch| {
                    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '@' | ':')
                })
            {
                return Err(
                    "client_id must be 1 to 128 characters of letters, digits and _ - . @ :",
                );
            }
        }
        match &self.credentials {
            Credentials::ApiKey { key } if key.is_empty() => Err("the api key must not be empty"),
            Credentials::Jwt { token } if token.is_empty() => Err("the token must not be empty"),
            Credentials::Basic { username, password }
                if username.is_empty() || password.is_empty() =>
            {
                Err("username and password must not be empty")
            }
            _ => Ok(()),
        }
    }
}

/// Server to client: the answer to [`Authenticate`]. Never carries a secret.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticationResult {
    pub request_id: String,
    pub success: bool,
    pub message: String,
    pub server_version: String,
    /// The identity this session's grants are filed under (the credential's subject).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// The scopes the session holds, e.g. `service:read` (informational).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_in_seconds: Option<u64>,
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

/// Client to server: give `client_id` permissions. With `service_id` they apply to that service only;
/// without it they are tenant-wide, shorthand for every service of the tenant.
/// Permission names are `tenant:read|write|admin`, `service:read|write|admin|resize` and so on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub request_id: String,
    pub client_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    pub permissions: Vec<String>,
}

impl Grant {
    /// Check the fields. A request that fails is answered `INVALID_REQUEST`.
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_grant_fields(&self.client_id, Some(&self.permissions), true)
    }
}

/// Client to server: take permissions away. An empty `permissions` list removes every permission of
/// `client_id` at that scope. Revoking a client from a **service** also triggers a revocation rotation
/// of that service alone: the client is removed from the file and gets `Loss{REVOKED}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revoke {
    pub request_id: String,
    pub client_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
}

impl Revoke {
    /// Check the fields. A request that fails is answered `INVALID_REQUEST`.
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_grant_fields(&self.client_id, Some(&self.permissions), false)
    }
}

fn validate_grant_fields(
    client_id: &str,
    permissions: Option<&[String]>,
    need_permissions: bool,
) -> Result<(), &'static str> {
    if client_id.trim().is_empty() {
        return Err("client_id must not be empty");
    }
    if let Some(p) = permissions {
        if need_permissions && p.is_empty() {
            return Err("permissions must not be empty");
        }
        if p.iter().any(|name| name.trim().is_empty()) {
            return Err("a permission name must not be blank");
        }
    }
    Ok(())
}

/// Server to client: the `Grant` or `Revoke` with this `request_id` took effect.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantAck {
    pub request_id: String,
}

/// Client to server: list grants, optionally of one client and/or one service.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListGrants {
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
}

/// One client's permissions at one scope (`service_id` absent = tenant-wide).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantEntry {
    pub client_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    pub permissions: Vec<String>,
}

/// Server to client: the answer to `ListGrants`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grants {
    pub request_id: String,
    pub entries: Vec<GrantEntry>,
}

/// Client to server: open (or close) a service to local mapping. Needs `service:admin` on the
/// service. A service is closed to local mapping until an administrator opens it, because the server
/// cannot see stores into a mapping: only operating-system file permissions guard it. While open, a
/// client with a write grant is offered `path` and `nonce_path` by `GetServiceFilePath`; closing it
/// stops future offers (an existing mapping ends at the next rotation).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetLocalMapping {
    pub request_id: String,
    pub service_id: String,
    pub enabled: bool,
}

/// Server to client: the `SetLocalMapping` with this `request_id` took effect.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMappingAck {
    pub request_id: String,
    pub service_id: String,
    pub enabled: bool,
}

/// Client to server: create a service of `size` bytes (rounded up to whole pages by the server).
/// Answered with `Service`. `request_id` is optional for compatibility with clients that predate it;
/// when present the reply echoes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateService {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub tenant_id: String,
    pub service_name: String,
    pub size: u64,
}

/// Client to server: look a service up by name. Answered with `Service`, or `Error NOT_FOUND`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetService {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub tenant_id: String,
    pub service_name: String,
}

/// Server to client: a service. The answer to `CreateService` (with `size`) and to `GetService`
/// (without it).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub service_id: String,
    pub service_name: String,
    pub tenant_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// Client to server: delete a service by name. Answered with `Result`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteService {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub tenant_id: String,
    pub service_name: String,
}

/// Client to server: create a tenant. Answered with `TenantResult`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateTenant {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub tenant_id: String,
    pub tenant_name: String,
}

/// Server to client: the answer to `CreateTenant`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub success: bool,
    pub tenant_id: String,
    pub message: String,
}

/// Client to server: delete a tenant with its services. Answered with `Result`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteTenant {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub tenant_id: String,
}

/// Server to client: the outcome of `DeleteService`, `DeleteTenant` or `Disconnect`. `request_id` is
/// the request's when it had one. Servers before this version put a fresh random id here, so a
/// client must match a `Result` only by an id it sent itself, never by presence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub success: bool,
    pub message: String,
}

/// Both directions: a keep-alive. The client sends it (no fields needed); the server answers with the
/// same type carrying `timestamp` (RFC 3339) and the request's `request_id` if it had one. Unknown
/// fields are ignored, as servers always have.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heartbeat {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Client to server: end the session. Answered with `Result`. Unknown fields are ignored.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disconnect {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Most audit entries one `ReadAudit` returns; a larger `limit` is clamped to this, not refused.
pub const MAX_AUDIT_LIMIT: u32 = 1000;
/// Entries returned when `ReadAudit.limit` is absent.
pub const DEFAULT_AUDIT_LIMIT: u32 = 100;

/// Client to server: read the audit events that concern the caller's tenant, oldest first, with
/// `seq` greater than `after`. Needs `tenant:admin`. `limit` defaults to [`DEFAULT_AUDIT_LIMIT`] and
/// is clamped to [`MAX_AUDIT_LIMIT`]; `0` is refused. Answered with `AuditEvents`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadAudit {
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl ReadAudit {
    /// Check the fields. A request that fails is answered `INVALID_REQUEST`.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.limit == Some(0) {
            return Err("limit must be at least 1");
        }
        Ok(())
    }

    /// The number of entries to return: the default if absent, else clamped to [`MAX_AUDIT_LIMIT`].
    pub fn effective_limit(&self) -> u32 {
        self.limit
            .unwrap_or(DEFAULT_AUDIT_LIMIT)
            .min(MAX_AUDIT_LIMIT)
    }
}

/// One audit event as stored. `hash` is recomputable from `(seq, ts_ms, prev, line)` (see
/// `spec/audit.md`); `prev` is the `hash` of the event with `seq - 1`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEntry {
    pub seq: u64,
    pub ts_ms: u64,
    pub line: String,
    pub prev: String,
    pub hash: String,
}

/// Why an audit entry did not verify (see `spec/audit.md`, Verification).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditHashError {
    /// The hash is well formed but is not the one the entry's fields produce.
    Mismatch,
    /// The hash names a formula this library does not know (or is not a hash at all). A verifier
    /// **refuses** such an entry; it never skips it or treats it as verified.
    UnsupportedVersion,
    /// The entry carries no hash (written before the chain existed).
    NotChained,
}

fn audit_hex(digest: impl AsRef<[u8]>) -> String {
    digest.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// The `commy-audit-v1` hash: 64 lower-case hex digits, no prefix. It does **not** cover the
/// event's tenant tag.
pub fn audit_hash_v1(seq: u64, ts_ms: u64, prev: &str, line: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"commy-audit-v1 ");
    h.update(seq.to_le_bytes());
    h.update(ts_ms.to_le_bytes());
    h.update((prev.len() as u64).to_le_bytes());
    h.update(prev.as_bytes());
    h.update(line.as_bytes());
    audit_hex(h.finalize())
}

/// The `commy-audit-v2` hash: `"v2:"` followed by 64 lower-case hex digits. Covers the tenant tag
/// (`None` is one zero byte; `Some` is a one byte, the tag's byte length as u64 little endian, then
/// the tag), placed after `prev` and before `line`.
pub fn audit_hash_v2(seq: u64, ts_ms: u64, prev: &str, tenant: Option<&str>, line: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"commy-audit-v2 ");
    h.update(seq.to_le_bytes());
    h.update(ts_ms.to_le_bytes());
    h.update((prev.len() as u64).to_le_bytes());
    h.update(prev.as_bytes());
    match tenant {
        None => h.update([0u8]),
        Some(t) => {
            h.update([1u8]);
            h.update((t.len() as u64).to_le_bytes());
            h.update(t.as_bytes());
        }
    }
    h.update(line.as_bytes());
    format!("v2:{}", audit_hex(h.finalize()))
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Recompute `entry.hash`. The hash's own prefix picks the formula: bare 64 hex is v1, `v2:` plus
/// 64 hex is v2, anything else is [`AuditHashError::UnsupportedVersion`] (never a guess).
/// `tenant` is the tenant the entry is being verified for (a reader's own tenant); v1 ignores it.
pub fn verify_audit_entry(entry: &AuditEntry, tenant: Option<&str>) -> Result<(), AuditHashError> {
    if entry.hash.is_empty() {
        return Err(AuditHashError::NotChained);
    }
    let want = if is_hex64(&entry.hash) {
        audit_hash_v1(entry.seq, entry.ts_ms, &entry.prev, &entry.line)
    } else if entry.hash.strip_prefix("v2:").is_some_and(is_hex64) {
        audit_hash_v2(entry.seq, entry.ts_ms, &entry.prev, tenant, &entry.line)
    } else {
        return Err(AuditHashError::UnsupportedVersion);
    };
    if want == entry.hash {
        Ok(())
    } else {
        Err(AuditHashError::Mismatch)
    }
}

/// The newest chained event of the whole log. It carries no tenant data: record it somewhere the
/// server cannot write to notice a log that was cut short.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditHead {
    pub seq: u64,
    pub hash: String,
}

/// Server to client: the answer to `ReadAudit`. `entries` are only events that concern the caller's
/// tenant. `more` is true when `limit` cut the list. `head` is absent when the server has no chained
/// event (or no persistence).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEvents {
    pub request_id: String,
    pub entries: Vec<AuditEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<AuditHead>,
    pub more: bool,
}

/// JSON control messages, sent as WebSocket text frames. Adjacently tagged
/// (`{"type": ..., "data": {...}}`); unknown fields and unknown types are refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[non_exhaustive]
pub enum ControlMessage {
    Authenticate(Authenticate),
    AuthenticationResult(AuthenticationResult),
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
    Grant(Grant),
    Revoke(Revoke),
    GrantAck(GrantAck),
    ListGrants(ListGrants),
    Grants(Grants),
    SetLocalMapping(SetLocalMapping),
    LocalMappingAck(LocalMappingAck),
    ReadAudit(ReadAudit),
    AuditEvents(AuditEvents),
    CreateService(CreateService),
    GetService(GetService),
    Service(Service),
    DeleteService(DeleteService),
    CreateTenant(CreateTenant),
    TenantResult(TenantResult),
    DeleteTenant(DeleteTenant),
    Result(OpResult),
    Heartbeat(Heartbeat),
    Disconnect(Disconnect),
}
