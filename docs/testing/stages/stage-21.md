# Stage 21: Server-Sent Events

Suite [`stage21-e2e.mjs`](../../../e2e/stage21-e2e.mjs) · 7 checks

A `text/event-stream` response shows event by event, an endless stream outlives the request timeout and can be stopped, and Run all does not hang on it. Creates "E2E Events" and deletes it afterwards, and restores the request timeout.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).

## Flow

### 1. A finite stream

- events show as they arrive, under an Events tab
- named events, ids and JSON data are shown
- the test script waits out event streams

### 2. An endless stream, past the timeout, stopped

- a stream outlives the request timeout
- Stop ends the stream and closes the connection

### 3. Run all

- Run all does not wait for streams and closes them
- test data cleaned up and the timeout put back

---

Previous: [Examples and the mock server](stage-20.md) · [Stage list](../README.md#stage-list) · Next: [gRPC requests](stage-22.md)
