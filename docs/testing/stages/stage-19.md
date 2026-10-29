# Stage 19: GraphQL requests

Suite [`stage19-e2e.mjs`](../../../e2e/stage19-e2e.mjs) · 10 checks

Tests the GraphQL card, variables with `{{variables}}`, fetching the schema, completion and validation in the editor, Copy as code, and Run all against a real GraphQL server. Creates "E2E GraphQL" and deletes it afterwards.

## Requirements

- A local GraphQL server built by the suite with the npm package `graphql`.

## Flow

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

Previous: [OpenAPI and Swagger import](stage-18.md) · [Stage list](../README.md#stage-list) · Next: [Examples and the mock server](stage-20.md)
