# Stage 35: Git fixes: a broken clone and a lost sign-in

Suite [`stage35-e2e.mjs`](../../../e2e/stage35-e2e.mjs) · 9 checks

Sync now rebuilds a clone that lost its `.git` folder, and a collection whose sign-in is gone offers a Sign in button. `https://auth.test/` points to a local Git HTTP server that answers 401 until the test "signs in". The Git credential helper is turned off for the test copy, so no sign-in window opens on the desktop.

## Requirements

- Git for Windows. `https://git.test/` is mapped to a local bare repository in `e2e/.work/git-hosts/` (see [harness](../README.md#harness)).
- A local Git HTTP server (`git http-backend`) started by the suite.

## Flow

### 1. A clone that lost its .git is rebuilt by Sync now

- the collection is shared
- Sync now rebuilds the lost clone and syncs
- the change saved here is sent and the collaborator's arrives
- no temporary clone is left behind

### 2. A lost sign-in offers Sign in

- a collection shares over the HTTP test host
- a missing sign-in shows sign in with a Sign in button
- Sign in that Git refuses says so
- once signed in, Sign in syncs and the button goes away
- test data cleaned up

---

Previous: [In-app updates](stage-34.md) · [Stage list](../README.md#stage-list) · Next: [Git changes with a long history](stage-36.md)
