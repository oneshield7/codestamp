# Stage 23: Orchestration

Suite [`stage23-e2e.mjs`](../../../e2e/stage23-e2e.mjs) · 16 checks

Builds a flow from Start to End on top of a local shop API (register, login, add to cart, checkout) and checks its step map: the value lines the selected step receives and passes on, arrow-key navigation, checks, a flow that stops (with a stop line to End) or continues on failure, order and saving, deleted requests, Quick search, and deleting the flow. Creates "E2E Shop" and deletes it afterwards.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).

## Flow

### 1. The page and a new flow

- the sidebar opens the Orchestration page
- a new flow shows Start, Add step and End on the map

### 2. Steps

- the picker offers HTTP requests but not WebSocket ones
- steps are numbered in order on the map, Add step sits before End
- the selected step shows the values it receives and passes on
- arrow keys move between steps on the map

### 3. A passing run

- a run goes from Start to End and every step passes
- values reach later steps: the token and the cart id

### 4. A failing check stops the flow

- a failed step stops the flow; later steps are skipped
- the failed step is selected and says why

### 5. Continue after a failure

- “Continue with the next step” keeps going after a failure

### 6. Reorder, persistence, deleted request

- steps move, and the flow is saved with its rules
- coming back to Orchestration shows a deleted request as a gap

### 7. Quick search and deleting the flow

- quick search finds the Orchestration page
- a flow can be deleted
- test data cleaned up

---

Previous: [gRPC requests](stage-22.md) · [Stage list](../README.md#stage-list) · Next: [XML and HTML bodies, SOAP, and WSDL import](stage-24.md)
