# Stage 21: Server-Sent Events

Suite [`stage21-e2e.mjs`](../../../e2e/stage21-e2e.mjs) · 7 cek

Response `text/event-stream` tampil event demi event, stream tanpa akhir bertahan melewati timeout request dan bisa dihentikan, dan Run all tidak macet karenanya. Membuat "E2E Events" lalu menghapusnya, dan mengembalikan timeout request seperti semula.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).

## Alur

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

Sebelumnya: [Example dan mock server](stage-20.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Request gRPC](stage-22.md)
