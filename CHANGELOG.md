# Changelog

## 0.4.1
- Repository metadata only: `.github/CODEOWNERS` (CodeRipper WSP-001). No code, spec, schema or vector change.

## 0.4.0 (breaking)
- `Authenticate` carries real credentials: `Credentials` (`api_key`, `jwt`, `basic`, `custom`, tagged by `method`, flattened beside the other fields), optional `client_version`, optional development-only `client_id` and `dev_scopes`; `Authenticate::validate`; secrets redacted from `Debug`. The earlier `auth_method` / `credentials` string pair is gone. New `AuthenticationResult`, `PROTOCOL_VERSION` (1). Spec section "Authentication", schema and vectors. R1 gate item 1: the wire freeze needs `Authenticate` and its result defined here, not ad hoc in the server.

## 0.3.2
- `SetLocalMapping` and `LocalMappingAck`: an administrator opens (or closes) a service to local mapping; `spec/control-messages.md`, schema and vectors. Additive.

## 0.3.1
- Service-scoped grants (C10 contract): `Grant`, `Revoke` (both with `validate`), `GrantAck`, `ListGrants`, `Grants`, `GrantEntry`; `spec/grants.md`; schema and vectors. Additive.

## 0.3.0 (breaking)
- Resize follows plan Q11(b), prepare and commit: the requester fills the new file. `RotationPending` gains `fill_ms` (required). New: `RotationPrepared` (server to the requester only), `RotationFilled`, `RotationAbort`, `RotationAborted`, `AbortReason`. `spec/rotation.md` is rewritten for RESIZE. The 0.2.x rotation messages described a resize in which the server copied `min(old, new)` bytes itself, which was not the approved design.

## 0.2.4
- Range forwarding messages (C3 contract): `RangesChanged`, `RangesUpdated`, `FlushEnd`, `ByteRange`, `validate_ranges`; `spec/forwarding.md`; schema and vectors. Additive.

## 0.2.3
- Rotation messages (C3/C4 contract): `RotateRequest` (with `validate`), `RotationAccepted`, `RotationPending`, `RotationDrained`, `RotationCommit`, `Loss`; `RotateReason`, `LossKind`, `LossRange`; `spec/rotation.md`; schema and vectors. Additive (variants join a `#[non_exhaustive]` enum).

## 0.2.2
- README: state that the layout is unfrozen.

## 0.2.1
- Local-mapping handshake: `GetServiceFilePath`, `ServiceFilePath` (path/nonce_path both or neither), `ConfirmLocal` (nonce redacted in Debug), `ConfirmLocalAck`; spec, schema and vectors. Variants added to a `#[non_exhaustive]` enum, so not breaking.

## 0.2.0
- `FrameError`, `ErrorCode` and `ControlMessage` are `#[non_exhaustive]`, so later variants are not breaking.
- Frame payload capped at 1 MiB (`MAX_PAYLOAD`, `FrameError::TooLarge`), `RangeFrame::split`, `ControlMessage` (Authenticate with protocol_version, FlushAck, Resync, ResyncDone, Error), `schema/control.schema.json`, `spec/control-messages.md` (multi-frame flushes, seq/echo rules, resync, reserved kinds).

## 0.1.0
- Draft-0 contract: binary range frame (`RangeFrame`, 25-byte little-endian header), `ErrorCode` (one SCREAMING_SNAKE spelling), JSON Schema for the error reply, golden vectors, `spec/range-frame.md`, `spec/errors.md`.
- CI (fmt, clippy, tests on Linux/Windows/macOS, doc, MSRV 1.94, cargo-deny) and `deny.toml`.

## 0.0.0
- Reserve the crate name.
