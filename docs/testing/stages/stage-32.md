# Stage 32: Sharing to a repository that already has files

Suite [`stage32-e2e.mjs`](../../../e2e/stage32-e2e.mjs) · 13 checks

Join explains the problem when a repository was not shared from Codestamp. Share lists the files for review before replacing them, and a repository that was replaced can be joined.

## Requirements

- Git for Windows. `https://git.test/` is mapped to a local bare repository in `e2e/.work/git-hosts/` (see [harness](../README.md#harness)).

## Flow

### 1. Join explains what is wrong with a repository Codestamp did not share

- Join says the repository was not shared from Codestamp

### 2. Share reviews the files first

- a repository with files is shown for review first
- nothing is pushed before the review is confirmed
- the review offers Cancel and Replace files and share
- Cancel goes back to the Share button
- editing the address drops the review
- a repository that moved after the review is not replaced
- checking again lists the new file too
- Replace files and share links the collection on the repository branch
- the old files are gone and the collection is in their place
- one ordinary commit on top keeps the old ones in history

### 3. The replaced repository can now be joined

- the replaced repository can be joined
- test data cleaned up

---

Previous: [Git accounts, Git changes, and replacing a collection](stage-31.md) · [Stage list](../README.md#stage-list) · Next: [Status bar, Ctrl+S, manual sync, and shared environments](stage-33.md)
