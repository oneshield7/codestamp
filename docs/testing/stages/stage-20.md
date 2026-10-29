# Stage 20: Examples and the mock server

Suite [`stage20-e2e.mjs`](../../../e2e/stage20-e2e.mjs) · 14 checks

Saves a response as an example, manages it in the Examples tab, starts a local mock server that answers with that example, sends a request through the mock from the app, and makes sure examples are exported as Collection v2.1 responses. Creates "E2E Mock" and deletes it afterwards.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).
- The Windows save or open file dialog, answered by `answer-dialog.ps1`.

## Flow

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

Previous: [GraphQL requests](stage-19.md) · [Stage list](../README.md#stage-list) · Next: [Server-Sent Events](stage-21.md)
