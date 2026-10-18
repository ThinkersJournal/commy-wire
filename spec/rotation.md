# Rotation: prepare, drain, commit (draft 1)

A rotation replaces a service's file with a new one at a new random name. The old mapping stays
valid until the program drops its guards. Two reasons exist (`RotateReason`): `RESIZE` and
`REVOCATION`. Rotation never replays old-offset writes across a resize, because Commy cannot know how
offsets map between layouts.

| Message          | Direction        | Fields |
|------------------|------------------|--------|
| `RotateRequest`  | client to server | `request_id`, `service_id`, `reason`, `new_size` (RESIZE only, at least 1), `revoke_client_id` (REVOCATION only) |
| `RotationAccepted` | server to requester | `request_id`, `rotation_id` |
| `RotationPending` | server to every SDK (RESIZE only) | `rotation_id`, `service_id`, `file_id`, `deadline_ms` |
| `RotationDrained` | client to server | `request_id`, `rotation_id`, `file_id`, `seq` |
| `RotationCommit` | server to every SDK | `rotation_id`, `service_id`, `reason`, `old_file_id`, `new_file_id`, `generation`, `size` |
| `Loss`           | server to one client | `service_id`, `file_id`, `kind`, optional `ranges` |

## Permissions (checked before anything is sent)

`RESIZE` needs `ServiceResize` on the service (`ServiceAdmin` implies it), and `new_size` may not
exceed the server maximum. `REVOCATION` needs `ServiceAdmin`. Failures are `PERMISSION_DENIED` or
`INVALID_REQUEST` (a request failing `RotateRequest::validate`, a size over the maximum or zero).
Rotation is rate limited per service and per requester (`RATE_LIMITED`).

## RESIZE

1. Server answers `RotationAccepted`, then sends `RotationPending` to every SDK on the service.
2. Each SDK stops issuing write guards, waits for outstanding ones, flushes into the old file, and
   sends `RotationDrained` with its last flushed `seq`, within `deadline_ms` (server default 5000).
3. When every SDK has drained, or the deadline passes, the server builds the new file of `new_size`
   (copying the old bytes that fit), and sends `RotationCommit` to every SDK that drained.
4. An SDK that did not drain gets `Loss{kind: ROTATION_STRAGGLER}` and is dropped from the service.
   Anything it wrote to the old mapping afterwards is lost, never replayed.
5. Each SDK fetches the new file with `GetServiceFilePath` (local) or `Resync` (remote), using
   `new_file_id`. `generation` increases by one per rotation.

## REVOCATION

There is no drain barrier. The server builds a new file of the same size and shape, keeping the old
bytes, and sends `RotationCommit` (`reason: REVOCATION`) to every SDK except the removed one. Each
remaining SDK writes its unflushed ranges into the new file after mapping it. The removed client gets
`Loss{kind: REVOKED}` and its unflushed writes are dropped.

## Audit

Every request, grant and refusal is written to the server's audit log.

## Timing, sequences and edge cases (clarifications)

- **`deadline_ms`** is relative to the SDK's receipt of `RotationPending`; no clocks are shared. An SDK
  that has not seen `RotationCommit` within `deadline_ms + 5000` ms of receiving `RotationPending`
  treats the rotation as failed, stops waiting, and resyncs. A blocked writer is never held forever.
- **`seq` restarts at 0 for every new `file_id`.** Frames carry `file_id`, so a receiver keys its
  last-applied-sequence table by `(file_id, seq)` and drops the old file's entries at commit.
  `RotationDrained.seq` is the highest `seq` this SDK sent for the old `file_id` (0 if it sent none).
- **After `RotationCommit`**, range frames for `old_file_id` are refused with `INVALID_REQUEST`.
  `GetServiceFilePath` returns `new_file_id` with a **fresh** nonce side file: the local handshake is
  redone for the new file and never reused. A REVOCATION survivor maps the new file **before**
  re-sending its unflushed ranges.
- **`Loss.ranges`** are `{offset, len}` in **old-file** coordinates; absent means the whole file.
  `LossKind` is `#[non_exhaustive]`: a receiver treats an unknown kind as a loss of the whole file.
- **Broadcasts** (`RotationPending`, `RotationCommit`) carry no `request_id`. A late joiner may receive
  a `RotationCommit` without a `RotationPending`: receivers must accept it. A second `RotationCommit`
  with the same `rotation_id` is ignored.
