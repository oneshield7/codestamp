# Stage 22: Request gRPC

Suite [`stage22-e2e.mjs`](../../../e2e/stage22-e2e.mjs) · 18 cek

Mengambil service lewat server reflection atau dari file `.proto` (dengan import), lalu menjalankan panggilan unary dan streaming dengan metadata dan auth, status error, serta memeriksa keadaan yang tersimpan. Membuat "E2E gRPC" lalu menghapusnya.

## Kebutuhan

- Server gRPC uji `grpc-shop-server.mjs` dengan paket `@grpc/grpc-js`, `@grpc/proto-loader`, dan `@grpc/reflection`.
- Dialog simpan atau buka file Windows, dijawab oleh `answer-dialog.ps1`.

## Alur

### 1. Awal

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

Sebelumnya: [Server-Sent Events](stage-21.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Orchestration](stage-23.md)
