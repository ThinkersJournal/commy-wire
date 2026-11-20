# Changelog

## 0.6.0 (commy R25; **Breaking**: `Credentials` no longer `non_exhaustive`, `Credentials::Mtls`; allocated by pm-assist 2026-11-20; before the R1 freeze)
- **Breaking: `Credentials::Mtls`** (`method: "mtls"`, no fields): the connection's verified TLS client certificate is the credential. `Credentials` is not `#[non_exhaustive]`, so an exhaustive `match` in a consumer stops compiling (the SDKs); on the wire an older server answers an unknown `method` with `INVALID_REQUEST`. Spec (Authentication), schema, a positive vector (the existing Authenticate vector count goes from 6 to 7). `Credentials::method()` returns `"mtls"`.
- **Breaking behaviour: `Authenticate` refuses a second method or a stray field** (pm-ants, 2026-11-20). It used to ignore any field its method did not define (the credential is flattened), so `mtls` plus a `password`, or `api_key` plus a `token`, parsed and the server silently took one. Now `Authenticate` has its own `Deserialize` (it checks the fields against the named method, then parses) and the request is `INVALID_REQUEST`. Five new invalid vectors in `vectors/control_messages.json`; every existing valid vector still parses. A client that sent extra fields (for example a leftover `auth_method`) is refused from this release. Schema: the Authenticate `data` object now lists the allowed fields per method.

## 0.5.2 (commy R30; allocated by pm-assist 2026-11-19)
- **`commy-audit-v2` audit hash** (`spec/audit.md`, "Hash versions"): covers the event's tenant tag, so a tag can no longer be changed by someone who can write the store without breaking the chain. The `hash` string's own prefix names the formula (bare hex = v1, `v2:`... = v2); a verifier refuses an unknown prefix, never skips it. v1 events keep verifying. Added `audit_hash_v1`, `audit_hash_v2`, `verify_audit_entry`, `AuditHashError`, and `vectors/audit_hash.json` (v1, v2, negative); new dependency `sha2`. Additive for the message types.
- **This minor must land before the R1 freeze**: it adds a hash formula the freeze would otherwise pin without the tag. Every SDK needs a [NOTICE] when it is cut.

## 0.5.1 (commy R24 and R1; allocated by pm-assist 2026-11-16)
- **Typed the service and tenant administration set** that servers already spoke as untyped JSON: `CreateService`, `GetService`, `Service`, `DeleteService`, `CreateTenant`, `TenantResult`, `DeleteTenant`, `Result` (`OpResult` in Rust), `Heartbeat`, `Disconnect`. Existing tags and field names are kept, so no current client breaks; `request_id` is optional on all of them and echoed in the reply when sent. Requests without it stay valid (vectors named `legacy:`), also past the freeze.
- **`ReadAudit` / `AuditEvents`** (commy R24, `spec/audit.md`): a tenant administrator reads the audit events tagged with their own tenant; `limit` default 100, clamped to 1000.
- Spec, schema and vectors for all of the above; the vectors and the schema agree in both directions (checked with a JSON Schema validator).

## 0.5.0 (breaking)
- `RotationCommit` no longer carries `generation` (owner, board 235: Commy does not track how many times a service has been rotated; `new_file_id` is the handle, and what format a service uses is the service's to say). Removed from the struct, `schema/control.schema.json`, `spec/rotation.md` and the vectors; a `RotationCommit` that still has the field is refused (`deny_unknown_fields`), and a vector pins that.
- Frame (1 MiB) and range-list (1024) limits unchanged.

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
