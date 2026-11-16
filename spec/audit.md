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
can recompute each entry's `hash`: SHA-256 over `"commy-audit-v1\0"`, `seq` (u64 little endian),
`ts_ms` (u64 little endian), the byte length of `prev` (u64 little endian), `prev`, then `line`,
written as lower-case hex. A match proves the entry was not edited. Recording `head` somewhere the
server cannot write proves later that the log was not cut short. Whole-chain verification is the
operator's.
