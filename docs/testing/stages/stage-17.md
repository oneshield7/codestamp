# Stage 17: Auth: Digest, AWS SigV4, dan OAuth 2.0

Suite [`stage17-e2e.mjs`](../../../e2e/stage17-e2e.mjs) · 15 cek

Server lokal memeriksa apa yang mereka terima untuk Digest, AWS Signature V4, dan OAuth 2.0: client credentials, password dengan refresh, authorization code dengan PKCE di jendela sign-in, dan auth yang diwarisi dari collection. Token harus ikut terhapus bersama request-nya. Membuat "E2E Auth" lalu menghapusnya.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).

## Alur

### 1. The servers

Menyalakan server lokal yang memeriksa permintaan auth.

### 2. CDP

Terhubung ke salinan uji lewat CDP dan memasang helper.

### 3. Digest

- Digest answers the 401 challenge
- Copy as code uses curl --digest
- a wrong Digest password shows the 401

### 4. AWS

- AWS Signature V4 signs path, sorted query and headers
- AWS Signature V4 signs a JSON body

### 5. OAuth: client credentials

- the Auth tab shows OAuth 2.0 with no token yet
- client credentials get a token with the Basic header
- an expired client-credentials token renews by itself
- Clear forgets the token

### 6. OAuth: password + refresh

- the password grant sends credentials in the body, then refreshes

### 7. OAuth: authorization code + PKCE

- authorization code with PKCE signs in through its own window
- the token goes out with the chosen header prefix

### 8. OAuth: inherited from the collection

- an inheriting request uses the collection’s token

### 9. Tokens go with their request

- deleting a request removes its token
- test data and its tokens cleaned up

---

Sebelumnya: [Path variable dan run berbasis data](stage-16.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Import OpenAPI dan Swagger](stage-18.md)
