# Stage 18: Import OpenAPI dan Swagger

Suite [`stage18-e2e.mjs`](../../../e2e/stage18-e2e.mjs) · 9 cek

Mengimpor spec OpenAPI 3 dari file YAML dan spec Swagger 2 dari URL menjadi collection, lalu mengirim salah satu request hasil import. Membuat "E2E Pets" dan "E2E Legacy" lalu menghapusnya.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).

## Alur

### 1. YAML file

- a YAML OpenAPI file previews as a collection
- operations become requests grouped by tag, with path variables
- the server becomes {{baseUrl}} and the API key a secret variable
- global security becomes collection auth; an empty list turns it off
- an imported request sends with its path variable, query and API key

### 2. Swagger from a URL

- a Swagger 2 spec can be fetched from a URL
- the Swagger operation is imported with its path variable
- plain http is refused for other hosts
- test data cleaned up

---

Sebelumnya: [Auth: Digest, AWS SigV4, dan OAuth 2.0](stage-17.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Request GraphQL](stage-19.md)
