# Stage 36: Git changes with a long history

Suite [`stage36-e2e.mjs`](../../../e2e/stage36-e2e.mjs) · 14 checks

A teammate adds 120 commits through a local bare repository. The Git changes page does not scroll as a whole; the commit list and its details scroll on their own under a fixed header. In a small window the content scrolls as a whole and older commits can still be loaded.

## Requirements

- Git for Windows. `https://git.test/` is mapped to a local bare repository in `e2e/.work/git-hosts/` (see [harness](../README.md#harness)).

## Flow

### 1. Setup

- in a small window the history scrolls as a whole
- older commits load in a small window too
- a commit far down shows its detail in a small window
- the collection is shared
- the collaborator's history arrives
- a long history does not make the page scroll
- the commit list scrolls on its own
- the panel ends inside the window
- Load older commits sits at the end of the list and comes into view
- scrolling the list keeps the picker and tabs in place
- older commits load into the same scroll area
- the detail of a commit far down shows without scrolling
- keyboard focus keeps the commit in view with room for its ring
- the Requests tab keeps the same fixed header

---

Previous: [Git fixes: a broken clone and a lost sign-in](stage-35.md) · [Stage list](../README.md#stage-list) · Next: [Access groups](stage-37.md)
