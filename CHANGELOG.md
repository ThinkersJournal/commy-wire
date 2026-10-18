# Changelog

## 0.1.1
- Frame payload capped at 1 MiB (`MAX_PAYLOAD`, `FrameError::TooLarge`), `RangeFrame::split`, `ControlMessage` (Authenticate with protocol_version, FlushAck, Resync, ResyncDone, Error), `schema/control.schema.json`, `spec/control-messages.md` (multi-frame flushes, seq/echo rules, resync, reserved kinds).

## 0.1.0
- Draft-0 contract: binary range frame (`RangeFrame`, 25-byte little-endian header), `ErrorCode` (one SCREAMING_SNAKE spelling), JSON Schema for the error reply, golden vectors, `spec/range-frame.md`, `spec/errors.md`.
- CI (fmt, clippy, tests on Linux/Windows/macOS, doc, MSRV 1.94, cargo-deny) and `deny.toml`.

## 0.0.0
- Reserve the crate name.
