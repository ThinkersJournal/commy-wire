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

## Local mapping handshake (draft 1)

Used only when the client and server share a filesystem. Any failure at any step means: stay on WSS
range frames. Nothing here weakens authentication: the session must already be authenticated.

| Step | Message | Direction | Notes |
|------|---------|-----------|-------|
| 1 | `GetServiceFilePath{request_id, service_id}` | client to server | Authenticated sessions only. |
| 2 | `ServiceFilePath{request_id, service_id, file_id, path?, nonce_path?}` | server to client | `file_id` is always present (range frames use it). `path` and `nonce_path` are present together or absent together; absent unless the service is open to local mapping (admin grant, default off) and the caller holds a write grant. |
| 3 | client opens `path` read-write and reads `nonce_path` | local | The nonce file is created per connection, mode 0600 on Unix, in the tenant directory (never in the service file). |
| 4 | `ConfirmLocal{request_id, file_id, nonce}` | client to server | The nonce is secret: never log it. |
| 5 | `ConfirmLocalAck{request_id, file_id}` or `Error PERMISSION_DENIED` | server to client | On match the server deletes the nonce file. A wrong nonce is refused and rate-limited. |

A file-size match or an IP match alone is not proof of the same filesystem view; the nonce is.
Read-only grantees never get a path: they use WSS even on the same machine.

### Opening a service to local mapping

`SetLocalMapping{request_id, service_id, enabled}` is answered by `LocalMappingAck{request_id,
service_id, enabled}`. It needs `service:admin` on the service (`PERMISSION_DENIED` otherwise,
`NOT_FOUND` for an unknown service). A service starts **closed**: until an administrator opens it,
`GetServiceFilePath` never offers `path` and `nonce_path`, whatever the caller's grants, because the
server cannot see stores into a mapping and cannot enforce write permission there. Closing stops
future offers; a mapping already open ends at the next rotation.
