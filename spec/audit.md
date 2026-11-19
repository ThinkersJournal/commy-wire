# Reading the audit log (draft 1)

A tenant administrator can read the audit events that concern their own tenant. The server's audit
log stays the server's own (hash-chained, durable); this is a reader, not a writer.

| Type | Direction | Fields |
|------|-----------|--------|
| `ReadAudit` | client to server | `request_id`, optional `after` (u64), optional `limit` (u32, at least 1) |
| `AuditEvents` | server to client | `request_id`, `entries` of `{seq, ts_ms, line, prev, hash}`, optional `head` `{seq, hash}`, `more` |

## Rules

- Needs `tenant:admin` on the caller's own tenant: `UNAUTHENTICATED` before `Authenticate`,
  `PERMISSION_DENIED` without it. Unknown fields and `limit` 0 are `INVALID_REQUEST`.
- `entries` are the events **tagged with the caller's tenant** with `seq` greater than `after`, oldest
  first. Events with no tenant tag (server-wide events, and every event written before a server tagged
  them) are **never** returned over the wire, and a tenant never sees another tenant's events.
- `limit` defaults to 100 and a larger value is **clamped** to 1000, not refused. `more` is true when
  `limit` cut the list: ask again with `after` set to the last `seq` returned.
- `head` is the newest chained event of the **whole** log (`seq`, `hash`); it carries no tenant data.
  It is absent when the server has no chained event or no persistence.
- No persistence: an empty `entries`, no `head`, `more` false.

## Verification

`seq` values in a tenant's view are not consecutive, so a client **cannot** re-run the whole chain. It
can recompute each entry's `hash`. A match proves the entry was not edited.

### Hash versions

The `hash` string itself says which formula made it; a verifier never guesses.

| `hash` looks like | Formula |
|---|---|
| 64 lower-case hex digits | **v1**: SHA-256 over `"commy-audit-v1\0"`, `seq` (u64 little endian), `ts_ms` (u64 little endian), the byte length of `prev` (u64 little endian), `prev`, then `line`, written as lower-case hex. Does **not** cover the tenant tag. |
| `v2:` then 64 lower-case hex digits | **v2**: SHA-256 over `"commy-audit-v2\0"`, `seq`, `ts_ms`, the byte length of `prev`, `prev` (all as in v1), then the tenant tag, then `line`; written as `v2:` plus lower-case hex. The tag is one byte `0x00` when the event has none, else `0x01`, the tag's UTF-8 byte length (u64 little endian) and the tag. Changing, adding or removing the tag breaks the hash. |

Rules:
- A verifier **refuses** an entry whose `hash` it does not recognise (for instance `v3:...`): an
  error, never "skipped" and never "verified". A v1-only verifier therefore refuses every v2 entry.
- Old events keep verifying under v1 for ever; a chain may mix versions. `prev` is always the previous
  event's whole `hash` string, prefix included.
- A reader verifies each entry for **its own tenant** (every entry it is served carries that tag).
  The tag is not a field of `AuditEntry`.
- An empty `hash` means the event predates the chain: not verifiable, not an error of the log.
- Vectors, including negative ones (re-tagged, tag removed, unknown prefix): `vectors/audit_hash.json`.
  Rust: `audit_hash_v1`, `audit_hash_v2`, `verify_audit_entry`.
 Recording `head` somewhere the
server cannot write proves later that the log was not cut short. Whole-chain verification is the
operator's.
