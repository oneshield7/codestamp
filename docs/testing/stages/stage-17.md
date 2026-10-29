# Stage 17: Auth: Digest, AWS SigV4, and OAuth 2.0

Suite [`stage17-e2e.mjs`](../../../e2e/stage17-e2e.mjs) · 15 checks

Local servers check what they receive for Digest, AWS Signature V4, and OAuth 2.0: client credentials, password with refresh, authorization code with PKCE in the sign-in window, and auth inherited from the collection. Tokens must be deleted together with their request. Creates "E2E Auth" and deletes it afterwards.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).

## Flow

### 1. The servers

Starts the local servers that check the auth requests.

### 2. CDP

Connects to the test copy over CDP and installs the helpers.

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

Previous: [Path variables and data-driven runs](stage-16.md) · [Stage list](../README.md#stage-list) · Next: [OpenAPI and Swagger import](stage-18.md)
