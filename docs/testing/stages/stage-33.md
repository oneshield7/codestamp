# Stage 33: Status bar, Ctrl+S, manual sync, and shared environments

Suite [`stage33-e2e.mjs`](../../../e2e/stage33-e2e.mjs) · 16 checks

The status bar holds Settings, Recently deleted, Cookies, and Environments. Ctrl+S saves environments and the collection panel, nothing syncs on its own without Sync now, collection variables overridden by the active environment are marked, and environments owned by a collection are shared with it.

## Requirements

- Git for Windows. `https://git.test/` is mapped to a local bare repository in `e2e/.work/git-hosts/` (see [harness](../README.md#harness)).

## Flow

### 1. Setup

- the collection is shared over the stand-in host

### 2. Note 5, moved on 2026-10-28: Settings and data pages sit in the status bar

- the status bar holds Settings, Recently deleted, Cookies and Environments
- the status bar runs along the bottom of the window
- its buttons open their pages

### 3. Note 2: Ctrl+S saves an environment

- Ctrl+S saves the environment being edited

### 4. Note 4: collection variables the active environment replaces

- a collection variable the active environment also has is marked replaced

### 5. Notes 2 and 3: Ctrl+S in the collection panel, and nothing syncs on its own

- Ctrl+S saves collection variables too
- a saved change waits for Sync now
- Sync now sends it

### 6. Note 1: an environment that belongs to the collection is shared

- Belongs to gives the environment to the collection
- the collection then has changes to sync
- Sync now writes it to environments/ without the secret value
- a collaborator’s environments and variables arrive with Sync now
- the secret value on this device is kept
- Git changes lists the environment files of a commit
- test data cleaned up

---

Previous: [Sharing to a repository that already has files](stage-32.md) · [Stage list](../README.md#stage-list) · Next: [In-app updates](stage-34.md)
