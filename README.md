# commy-wire

The wire contract between the Commy server and its SDKs: spec, JSON Schema, golden message vectors and the shared Rust wire types.

Name reserved (0.0.0 placeholder). The first real release follows the Commy plan; its version will match the Commy release it belongs to.

Licensed under MIT OR Apache-2.0.

## Layout

- `spec/` the human-readable contract (`range-frame.md`, `errors.md`), draft 0
- `schema/` JSON Schema for control messages (`error.schema.json` so far)
- `vectors/` golden vectors every SDK runs in CI (`range_frame.json`, `error_codes.json`)
- `src/` the Rust types, tested against the same vector files

## Messages

Typed (Rust type, JSON Schema, golden vectors, spec): everything in `spec/control-messages.md`, `spec/grants.md`, `spec/rotation.md` and `spec/audit.md`: authentication, flushes and ranges, resync, local mapping, rotation, grants, `ReadAudit` / `AuditEvents`, and the service and tenant administration set (`CreateService`, `GetService`, `Service`, `DeleteService`, `CreateTenant`, `TenantResult`, `DeleteTenant`, `Result`, `Heartbeat`, `Disconnect`).

Accepted legacy: requests from clients that predate `request_id` on the administration set (no `request_id`) stay valid; they are vectors in `vectors/control_messages.json` named `legacy: ...`. Removing them would be a later major version with its own notice.

## Status: layout unfrozen

The wire layout is **not frozen**. Until the freeze (before 1.0), any layout change is the next minor version, and older versions stay available on crates.io. Pin an exact version and run the golden vectors in `vectors/` in your CI.
