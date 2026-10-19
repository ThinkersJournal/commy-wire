# Rotation: prepare, drain, commit (draft 1)

A rotation replaces a service's file with a new one at a new random name. The old mapping stays
valid until the program drops its guards. Two reasons exist (`RotateReason`): `RESIZE` and
`REVOCATION`. Rotation never replays old-offset writes across a resize, because Commy cannot know how
offsets map between layouts.

| Message          | Direction        | Fields |
|------------------|------------------|--------|
| `RotateRequest`  | client to server | `request_id`, `service_id`, `reason`, `new_size` (RESIZE only, at least 1), `revoke_client_id` (REVOCATION only) |
| `RotationAccepted` | server to requester | `request_id`, `rotation_id` |
| `RotationPending` | server to every SDK (RESIZE only) | `rotation_id`, `service_id`, `file_id`, `deadline_ms` (drain), `fill_ms` (fill budget) |
| `RotationDrained` | client to server | `request_id`, `rotation_id`, `file_id`, `seq` |
| `RotationPrepared` | server to the requester only (RESIZE only) | `rotation_id`, `service_id`, `old_file_id`, `new_file_id`, `size`, optional `path` |
| `RotationFilled` | requester to server | `request_id`, `rotation_id` |
| `RotationAbort` | requester to server | `request_id`, `rotation_id` |
| `RotationAborted` | server to every member and the requester | `rotation_id`, `service_id`, `file_id`, `reason` |
| `RotationCommit` | server to every SDK | `rotation_id`, `service_id`, `reason`, `old_file_id`, `new_file_id`, `generation`, `size` |
| `Loss`           | server to one client | `service_id`, `file_id`, `kind`, optional `ranges` |

## Permissions (checked before anything is sent)

`RESIZE` needs `ServiceResize` on the service (`ServiceAdmin` implies it), and `new_size` may not
exceed the server maximum. `REVOCATION` needs `ServiceAdmin`. Failures are `PERMISSION_DENIED` or
`INVALID_REQUEST` (a request failing `RotateRequest::validate`, a size over the maximum or zero).
Rotation is rate limited per service and per requester (`RATE_LIMITED`).

## RESIZE (prepare and commit)

Only the service knows how to migrate its own layout, so **the requester fills the new file**; the
server never copies old bytes for a resize.

1. Server answers `RotationAccepted`, then sends `RotationPending` to every SDK on the service.
2. Each SDK stops issuing write guards, waits for outstanding ones, flushes into the old file, and
   sends `RotationDrained` with its last flushed `seq`, within `deadline_ms` (server default 5000).
3. When every SDK has drained, or the deadline passes, the server creates the new file of `new_size`
   at a new random name, **zero filled and not live**, and sends `RotationPrepared` **to the requester
   only**. A local requester gets `path` and writes the file directly; a remote requester sends range
   frames for `new_file_id` (the server writes them into the prepared file) ending with `FlushEnd`,
   answered by `FlushAck` with `seq` 0. No other client learns the new file's path or id yet.
4. The requester reads the old file in the old shape, writes the new file in the new shape, and sends
   `RotationFilled`. The server then makes the new file live and sends `RotationCommit` to every
   member that drained (and to the requester). Writers that were waiting resume on the new file.
5. A member that did not drain gets `Loss{kind: ROTATION_STRAGGLER}` and is dropped from the service.
   Anything it wrote to the old mapping afterwards is lost, never replayed.
6. Each SDK fetches the new file with `GetServiceFilePath` (local) or `Resync` (remote), using
   `new_file_id`. `generation` increases by one per rotation.

**Abort.** If the requester sends `RotationAbort`, or sends nothing within `fill_ms` of
`RotationPrepared`, the server discards the prepared file, keeps the old one live, and sends
`RotationAborted{reason}` to every member and the requester. Writers resume on the old file; nothing is
replayed or lost. If the server cannot build or switch to the file it aborts with `SERVER_ERROR`.
`reason` is `REQUESTER_ABORT`, `FILL_TIMEOUT`, `SERVER_ERROR`, or an unknown value from a newer server,
which a receiver treats as an abandoned rotation (Rust: `AbortReason::Unknown`).

## REVOCATION

There is no drain barrier. The server builds a new file of the same size and shape, keeping the old
bytes, and sends `RotationCommit` (`reason: REVOCATION`) to every SDK except the removed one. Each
remaining SDK writes its unflushed ranges into the new file after mapping it. The removed client gets
`Loss{kind: REVOKED}` and its unflushed writes are dropped.

## Audit

Every request, grant and refusal is written to the server's audit log.

## Timing, sequences and edge cases (clarifications)

- **`deadline_ms`** (drain) and **`fill_ms`** (fill) are relative to the SDK's receipt of
  `RotationPending`; no clocks are shared. An SDK that has seen neither `RotationCommit` nor
  `RotationAborted` within `deadline_ms + fill_ms + 5000` ms of receiving `RotationPending` treats the
  rotation as failed, stops waiting, and resyncs. A blocked writer is never held forever.
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

The Rust type carries this as `LossKind::Unknown` (deserialized from any unrecognised kind); the JSON
Schema therefore accepts any string for `Loss.kind`. Vectors: `tolerated_unknown_loss_kinds` in
`vectors/control_messages.json`.

## Prepare and commit: clarifications (SDK review)

- **Terminal state.** A `rotation_id` ends at the **first** of `RotationCommit` or `RotationAborted`.
  A receiver ignores any later message carrying the same `rotation_id` (a second commit, or an abort
  after a commit, or the reverse).
- **Abort and stragglers.** An abort drops no one. Members that missed the drain deadline are **kept**:
  nothing was switched, so nothing was lost. `ROTATION_STRAGGLER` is sent only at a commit. After an
  abort every member, drained or not, resumes writing to the old file.
- **Remote fill.** Range frames for `new_file_id` arrive before any `Resync` or mirror of that file
  exists. Their offsets are **absolute positions in the new file**. `seq` restarts at 0 for the new file,
  and the `FlushEnd` is answered by a `FlushAck` that echoes its `request_id` with `seq` 0; that 0 is the
  new file's first sequence number and is never "stale".
- **Local fill.** The requester opens `path` under the same rules as `GetServiceFilePath` (a regular,
  non-symlink file) and writes it directly. No nonce handshake is needed to fill; the handshake for the
  new file happens after the commit, through `GetServiceFilePath`.
- **Requester timing.** The requester is subject to the drain deadline like every member: a requester
  that is a member must send `RotationDrained` within `deadline_ms`. The server sends
  `RotationPrepared` when the barrier ends whether or not the requester drained, so a requester that
  did not drain can still fill the file, but it is then dropped as a straggler at the commit.
  `fill_ms` runs from the SDK's receipt of `RotationPrepared`.
