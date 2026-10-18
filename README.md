# commy-wire

The wire contract between the Commy server and its SDKs: spec, JSON Schema, golden message vectors and the shared Rust wire types.

Name reserved (0.0.0 placeholder). The first real release follows the Commy plan; its version will match the Commy release it belongs to.

Licensed under MIT OR Apache-2.0.

## Layout

- `spec/` the human-readable contract (`range-frame.md`, `errors.md`), draft 0
- `schema/` JSON Schema for control messages (`error.schema.json` so far)
- `vectors/` golden vectors every SDK runs in CI (`range_frame.json`, `error_codes.json`)
- `src/` the Rust types, tested against the same vector files
