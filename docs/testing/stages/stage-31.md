# Stage 31: Git accounts, Git changes, and replacing a collection

Suite [`stage31-e2e.mjs`](../../../e2e/stage31-e2e.mjs) · 20 checks

Tests Settings › Git accounts, the account choice in Collaboration, Tools › Git changes, and Import › replace for a shared collection.

## Requirements

- Git for Windows. `https://git.test/` is mapped to a local bare repository in `e2e/.work/git-hosts/` (see [harness](../README.md#harness)).

## Flow

### 1. Setup

- the collection is shared over the stand-in https host

### 2. Settings › Git accounts

- the account a collection uses is listed with it
- Add account adds a labelled account not used yet
- the same account cannot be added twice
- Check reaches the repository with that account

### 3. Collaboration picks from the list

- Change account offers the saved accounts of the host
- an unused account can be removed

### 4. Tools › Git changes

- Git changes opens on the shared collection with its local edit
- Sync now adds the commit to the list, under Today
- the commit shows the request and the field it changed
- before and after are side by side
- the first commit lists what Share added
- the Requests tab shows who changed each request and its versions
- History opens the versions of the request
- arrow keys move between the tabs

### 5. Import › replace the shared collection

- Import offers to replace an existing collection
- replacing asks first and says what goes
- the collection keeps its name and link, with the file’s requests
- Sync now sends the replacement as one ordinary commit, not before
- test data cleaned up

---

Previous: [A Git account per collection](stage-30.md) · [Stage list](../README.md#stage-list) · Next: [Sharing to a repository that already has files](stage-32.md)
