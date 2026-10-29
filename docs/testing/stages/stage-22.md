# Stage 22: gRPC requests

Suite [`stage22-e2e.mjs`](../../../e2e/stage22-e2e.mjs) · 18 checks

Loads services through server reflection or from a `.proto` file (with imports), then runs unary and streaming calls with metadata and auth, error statuses, and checks the saved state. Creates "E2E gRPC" and deletes it afterwards.

## Requirements

- The gRPC test server `grpc-shop-server.mjs` with the packages `@grpc/grpc-js`, `@grpc/proto-loader`, and `@grpc/reflection`.
- The Windows save or open file dialog, answered by `answer-dialog.ps1`.

## Flow

### 1. Setup

- collection variables are set for the test
- rows are stamped gRPC and Run all is off for gRPC only

### 2. Server reflection

- the card shows gRPC, Invoke, and no code or capture tools
- reflection lists every method with its kind of stream
- choosing a method fills in a message with every field
- an untouched template follows the method; client streams get an array

### 3. Unary with metadata and auth

- a unary call answers OK with the message
- metadata and Bearer auth (from a secret variable) reach the server
- response headers and trailers are shown apart

### 4. Streams

- a server stream shows each message
- a client stream sends every array element
- a bidirectional stream sends and reads both ways

### 5. Failures

- an error status is shown with its message
- a message that does not fit the type is refused before sending

### 6. A .proto file

- a .proto path not picked on this device is not read
- a picked .proto file (with an import beside it) describes the service

### 7. Saved state

- the chosen method is saved with the request
- test data cleaned up

---

Previous: [Server-Sent Events](stage-21.md) · [Stage list](../README.md#stage-list) · Next: [Orchestration](stage-23.md)
