# Stage 30: Akun Git per collection

Suite [`stage30-e2e.mjs`](../../../e2e/stage30-e2e.mjs) · 26 cek

Panel Collaboration meminta akun hanya bila dipakai, dua collection memakai dua author berbeda, akun satu collection bisa diganti, collection https punya akun sign-in sendiri, dan backend ikut menjaga nilai akun. Dua bare repository lokal berperan sebagai dua host Git.

## Kebutuhan

- Git for Windows. `https://git.test/` dipetakan ke bare repository lokal di `e2e/.work/git-hosts/` (lihat [harness](../README.md#harness)).

## Alur

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

Sebelumnya: [Find di editor, nama baru, dan Docs yang ringkas](stage-29.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Git accounts, Git changes, dan replace collection](stage-31.md)
