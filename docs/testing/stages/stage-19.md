# Stage 19: Request GraphQL

Suite [`stage19-e2e.mjs`](../../../e2e/stage19-e2e.mjs) · 10 cek

Menguji kartu GraphQL, variabel dengan `{{variabel}}`, mengambil schema, completion dan pemeriksaan di editor, Copy as code, dan Run all terhadap server GraphQL sungguhan. Membuat "E2E GraphQL" lalu menghapusnya.

## Kebutuhan

- Server GraphQL lokal yang dibuat suite dengan paket npm `graphql`.

## Alur

### 1. The card

- Add GraphQL makes a GQL card with a Query tab
- variables with {{variables}} go out as GraphQL JSON

### 2. Schema, completion, checks

- Fetch schema loads the types
- the editor completes fields from the schema
- unknown fields are marked as errors
- variables that are not JSON are refused
- blank variables are left out

### 3. Copy as code

- Copy as code sends the query as JSON with POST

### 4. Run all

- Run all sends GraphQL requests
- test data cleaned up

---

Sebelumnya: [Import OpenAPI dan Swagger](stage-18.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Example dan mock server](stage-20.md)
