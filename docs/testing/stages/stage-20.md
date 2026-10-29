# Stage 20: Example dan mock server

Suite [`stage20-e2e.mjs`](../../../e2e/stage20-e2e.mjs) · 14 cek

Menyimpan response sebagai example, mengelolanya di tab Examples, menjalankan mock server lokal yang menjawab dengan example itu, mengirim request lewat mock dari aplikasi, dan memastikan example ikut terekspor sebagai response Collection v2.1. Membuat "E2E Mock" lalu menghapusnya.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).
- Dialog simpan atau buka file Windows, dijawab oleh `answer-dialog.ps1`.

## Alur

### 1. Save as example

- a response is saved as an example and listed
- examples can be renamed
- an example opens with its headers and body

### 2. Mock server

- the mock server starts on 127.0.0.1
- a matching path answers with the first successful example
- x-mock-response-code picks another example
- x-mock-response-name picks by name
- an unmatched request gets a clear 404
- the panel logs what the mock answered

### 3. Send through the mock from the app

- requests sent from Codestamp reach the mock

### 4. Export

- examples are exported as Collection v2.1 responses

### 5. Stop and delete

- Stop closes the mock server
- an example can be deleted
- test data cleaned up

---

Sebelumnya: [Request GraphQL](stage-19.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Server-Sent Events](stage-21.md)
