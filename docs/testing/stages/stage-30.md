# Stage 30: A Git account per collection

Suite [`stage30-e2e.mjs`](../../../e2e/stage30-e2e.mjs) · 26 checks

The Collaboration panel asks for an account only when one is needed, two collections use two different authors, one collection's account can be changed, an https collection has its own sign-in account, and the backend guards the account values too. Two local bare repositories act as two Git hosts.

## Requirements

- Git for Windows. `https://git.test/` is mapped to a local bare repository in `e2e/.work/git-hosts/` (see [harness](../README.md#harness)).

## Flow

### 1. The share form asks for an account only where one is used

- the Git account field shows for https addresses only
- the field says what it is for and what empty means
- a username with a space is refused before Git runs
- a valid username goes on to Git, which reports the unreachable host
- the failed share leaves no link and no clone

### 2. Two collections, two authors

- a shared collection shows the author it commits as
- collection A is pushed as its author
- the next share starts from the last author used
- collection B is pushed as its own author

### 3. Change the account of one collection

- Change account opens a form filled with this collection’s account
- Save waits for a name
- Save shows the new account
- the next sync of B commits as the new account
- collection A keeps its own account
- and its next sync still commits as Ana Work

### 4. An https collection with its own sign-in account

- an https collection shares with its own Git account
- its account form picks the Git account from the saved list
- Another account… switches to a text box
- a new Git account is saved without syncing on its own
- Sync now then uses the new account
- the list holds both accounts of this host
- an empty Git account leaves the choice to Git

### 5. The backend guards the account as well

- a username for a folder repository is refused
- an invalid email is refused
- each link reports its own author
- test data cleaned up

---

Previous: [Find in the editor, new names, and a compact Docs page](stage-29.md) · [Stage list](../README.md#stage-list) · Next: [Git accounts, Git changes, and replacing a collection](stage-31.md)
