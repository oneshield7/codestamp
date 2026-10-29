# Collections: Basic collection flow

Suite [`collections-e2e.mjs`](../../../e2e/collections-e2e.mjs) · 8 checks

Creates and renames a collection, opens a new request as a card, waits for autosave, sends to a local server and views the formatted response body, makes sure a folded row keeps its last status, and makes sure the request is still there after a reload. The test collection is deleted at the end.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).

## Flow

### 1. Setup

- collection created and renamed
- new request opens as a card
- autosave finishes
- send returns 200 from local server
- response body shown and formatted
- collapsed row keeps last status
- request persists after reload
- test collection cleaned up

---

Previous: [Quick search, shortcuts, status bar, Export, History, and Recently deleted](stage-40.md) · [Stage list](../README.md#stage-list) · Next: [codestamp-cli](cli.md)
