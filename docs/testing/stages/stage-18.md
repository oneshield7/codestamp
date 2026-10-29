# Stage 18: OpenAPI and Swagger import

Suite [`stage18-e2e.mjs`](../../../e2e/stage18-e2e.mjs) · 9 checks

Imports an OpenAPI 3 spec from a YAML file and a Swagger 2 spec from a URL as collections, then sends one of the imported requests. Creates "E2E Pets" and "E2E Legacy" and deletes them afterwards.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).

## Flow

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

Previous: [Auth: Digest, AWS SigV4, and OAuth 2.0](stage-17.md) · [Stage list](../README.md#stage-list) · Next: [GraphQL requests](stage-19.md)
