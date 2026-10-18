# Control messages and flush rules (draft 1)

JSON **text** frames, adjacently tagged: `{"type": <Name>, "data": {...}}`. Unknown types and unknown
fields are refused with `INVALID_REQUEST`. Every request carries a `request_id`; the reply echoes it.
Schema: `schema/control.schema.json`. Vectors: `vectors/control_messages.json`.

| Type           | Direction        | Fields                                                                  |
|----------------|------------------|-------------------------------------------------------------------------|
| `Authenticate` | client to server | `request_id`, `protocol_version` (u32, currently 1), `tenant_id`, `client_id`, `auth_method`, `credentials` |
| `FlushAck`     | server to client | `request_id`, `file_id`, `seq` (new sequence after the flush)           |
| `Resync`       | client to server | `request_id`, `file_id`                                                  |
| `ResyncDone`   | server to client | `request_id`, `file_id`, `seq`, `size`                                   |
| `Error`        | server to client | `code`, `message`, optional `request_id`                                |

## Frame size and multi-frame flushes

- A range frame carries at most **1 MiB** (1,048,576 bytes) of payload; a larger `len` is refused
  (`TooLarge`). A bigger change is split into consecutive frames, in offset order.
- One flush may span several frames. **All frames of one flush share one `seq`.**
- Frames are applied as they arrive. There is **no atomicity across frames** (or across ranges): readers
  converge to the latest bytes, as in the flush contract.

## Ordering and echo

- A receiver keeps the last applied `seq` per `file_id`. A frame with `seq` **lower** than that is
  dropped. An **equal** `seq` is applied (it is another frame of the same flush).
- The server **never sends a writer its own ranges back.** An SDK that applies an inbound range updates
  its baseline in the same step, so the range is not treated as a local change and is not re-sent.

## Resync (snapshot)

There is no replay log in v1. On a sequence gap, a reconnect or an over-threshold pending set, the SDK
sends `Resync`. The server answers with the whole file as range frames (each at most 1 MiB, all with
the same `seq`), then `ResyncDone` carrying that final `seq` and the file `size`. Only after
`ResyncDone` does the SDK treat the file as current; it then re-sends ranges that are still locally dirty.

## Reserved frame kinds

Kind `1` is the range frame. Kinds `2` to `255` are **reserved**; a receiver **refuses** a frame with
an unknown kind (`BadKind`) rather than ignoring it, so a protocol mismatch fails loudly.
