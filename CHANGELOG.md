# Changelog

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
