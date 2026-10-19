# Service-scoped grants (draft 1)

Access is granted **per client, tenant and service**. A tenant-wide grant (no `service_id`) stays as
shorthand for every service of the tenant. The server checks the grant before it processes any
request that names a service or a file of that service (`GetServiceFilePath`, `Resync`, `RangesChanged`,
range frames, `FlushEnd`, `RotateRequest`, and every administration message here).

| Message | Direction | Fields |
|---------|-----------|--------|
| `Grant` | client to server | `request_id`, `client_id`, `service_id` (absent = tenant-wide), `permissions` (at least one) |
| `Revoke` | client to server | `request_id`, `client_id`, `service_id` (absent = tenant-wide), `permissions` (empty = all at that scope) |
| `GrantAck` | server to client | `request_id` |
| `ListGrants` | client to server | `request_id`, optional `client_id`, optional `service_id` |
| `Grants` | server to client | `request_id`, `entries` of `{client_id, service_id?, permissions}` |

## Permissions

Names are `service:read`, `service:write`, `service:resize`, `service:admin` at service scope, and
the same plus `tenant:read`, `tenant:write`, `tenant:admin` at tenant scope. `service:admin` implies
`service:write`, `service:read` and `service:resize`; `service:write` implies `service:read`. A
tenant-wide grant of a permission applies to every service. Unknown names are refused with
`INVALID_REQUEST`.

## Who may change grants

Granting or revoking at **service** scope needs `service:admin` on that service (or tenant-wide
`tenant:admin`). At **tenant** scope it needs `tenant:admin`. A client cannot grant more than it
holds. Failures are `PERMISSION_DENIED`. Every grant, revoke and refusal is written to the audit log.

## Revocation closes the gap

Revoking a client's last access to a service (`Revoke` with a `service_id`, or the last permission
removed) triggers a **revocation rotation** of that service only (`spec/rotation.md`): the file moves
to a new random name, the remaining members get `RotationCommit`, and the revoked client gets
`Loss{REVOKED}` and can no longer fetch the new file because it holds no grant. A tenant-wide revoke
rotates every service the client could still reach only through that grant.

`ListGrants` returns only what the caller may see: a service admin sees that service's grants, a
tenant admin sees all, anyone else sees their own entries.
