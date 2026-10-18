# Error codes (draft 0)

One spelling, `SCREAMING_SNAKE_CASE`, from the server and every SDK. Anything else is invalid.

| Code                  | Meaning                                                              |
|-----------------------|----------------------------------------------------------------------|
| `INVALID_REQUEST`     | Malformed message, bad field, bad frame, bad tenant id.              |
| `UNAUTHENTICATED`     | No valid session. Replaces the old `Unauthorized` for this case.     |
| `PERMISSION_DENIED`   | Authenticated, but lacking the grant (service-scoped).               |
| `NOT_FOUND`           | The tenant, service or file does not exist.                          |
| `RATE_LIMITED`        | A rotation or request limit was hit.                                 |
| `UNSUPPORTED_VERSION` | The protocol version sent in `Authenticate` is not supported.        |
| `INTERNAL_ERROR`      | A server fault; details are logged, not sent.                        |

An error reply is `{"type":"Error","data":{"code":<CODE>,"message":<text>,"request_id":<echoed>}}`.
Schema: `schema/error.schema.json`. Vectors: `vectors/error_codes.json`.
