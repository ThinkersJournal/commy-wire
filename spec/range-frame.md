# Binary range frame (draft 0)

Sent as a WebSocket **binary** message. Control messages are JSON **text** frames; a binary frame is
never JSON. All integers are **little-endian**.

| Offset | Size | Field     | Meaning                                                              |
|-------:|-----:|-----------|----------------------------------------------------------------------|
| 0      | 1    | `kind`    | `1` = range. Any other value is refused (`BadKind`).                  |
| 1      | 4    | `file_id` | Service file, assigned by the server when the service is opened.     |
| 5      | 8    | `seq`     | Per-service sequence number (see the flush contract).                |
| 13     | 8    | `offset`  | Byte offset of the first changed byte in the file.                   |
| 21     | 4    | `len`     | Number of payload bytes, 1 or more.                                  |
| 25     | len  | payload   | The changed bytes.                                                   |

A receiver refuses a frame when: fewer than 25 bytes arrive (`Truncated`); `len` differs from the
payload byte count (`LengthMismatch`); `len` is 0 (`EmptyRange`); `offset + len` overflows a u64
(`OffsetOverflow`). A refused frame is answered with `INVALID_REQUEST` and is never partly applied.

`seq` semantics, coalescing and resync are in the flush contract (plan section A); this document
fixes only the byte layout. Golden vectors: `vectors/range_frame.json`.

Status: **draft 0**, not frozen. The layout freezes when the first coordinated release is cut.
