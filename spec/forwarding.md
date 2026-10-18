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

**Range lists** hold 1 to **1024** ranges, sorted by offset, non-empty, non-overlapping (adjacent is
fine). A flush with more ranges is split into several `RangesChanged` messages; each is its own flush
and gets its own `seq`. Every SDK splits at 1024 so the results match.

## Receiving

**A `RangesUpdated` obliges the receiving SDK to re-baseline those ranges from the live bytes.** The
bytes are already in the shared file, so if the SDK's scan or twin still holds the old bytes, its next
commit reports the ranges as local changes and sends them back: an echo loop. *Known race:* an
unflushed local write that overlaps a `RangesUpdated` range is absorbed (the last writer to the
mapping wins). This is documented behaviour and cannot be detected.

| Receiver | Gets |
|----------|------|
| **Local** | `RangesUpdated{file_id, seq, ranges}`, no data |
| **Remote** | binary range frames with the bytes read from the file **at forwarding time**, all carrying the flush's `seq` |

The sender is never sent its own ranges back. A `FlushAck` returns the `seq` the sender's next frames
use as their base. An ack `seq` lower than the last applied for that `file_id` is legal (another
sender's flush may have been applied first); the SDK keeps the maximum. A receiver applies a frame only if its `seq` is at least
the last applied for that `file_id` (see `control-messages.md`).

## Backpressure

The server holds, per remote receiver, one coalesced dirty-range set bounded by the file size, never
a queue. A receiver that falls behind gets a resync instead of a backlog.
