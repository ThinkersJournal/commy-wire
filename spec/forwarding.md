# Range forwarding (draft 1)

The server keeps one file per service and one `seq` per file (restarting at 0 for each new `file_id`).
Every flush from any SDK gets exactly one new `seq`, assigned by the server: the server's order wins.
Receivers converge to the **latest bytes**; they do not see every intermediate value.

## Sending

| Sender | How | Server does |
|--------|-----|-------------|
| **Local** (maps the file) | `RangesChanged{request_id, file_id, ranges}`, no data | validates, assigns `seq`, forwards, answers `FlushAck{request_id, file_id, seq}` |
| **Remote** (has a mirror) | one or more binary range frames, then `FlushEnd{request_id, file_id}` | writes each frame's bytes into the file as it arrives; at `FlushEnd` assigns `seq`, forwards the flush's ranges, answers `FlushAck` |

Both need a write grant on the service (`PERMISSION_DENIED` otherwise) and a file in the sender's
tenant (`NOT_FOUND` otherwise; another tenant's file is never confirmed to exist). The frame's `seq`
field is the base sequence the sender last saw; the server records it but does not reject on it.
Frames for a file whose rotation has committed are refused (`INVALID_REQUEST`).
A range list must pass `validate_ranges`, and every range must lie inside the file.

## Receiving

| Receiver | Gets |
|----------|------|
| **Local** | `RangesUpdated{file_id, seq, ranges}`, no data |
| **Remote** | binary range frames with the bytes read from the file **at forwarding time**, all carrying the flush's `seq` |

The sender is never sent its own ranges back. A receiver applies a frame only if its `seq` is at least
the last applied for that `file_id` (see `control-messages.md`).

## Backpressure

The server holds, per remote receiver, one coalesced dirty-range set bounded by the file size, never
a queue. A receiver that falls behind gets a resync instead of a backlog.
