# Stage 16: Path variable dan run berbasis data

Suite [`stage16-e2e.mjs`](../../../e2e/stage16-e2e.mjs) · 15 cek

Menguji path variable (`/users/:id`) di kartu request, saat mengirim, di pratinjau URL, serta saat export dan import, lalu menjalankan collection dengan file data CSV. Membuat "E2E Paths" dan "E2E Paths Import" lalu menghapusnya.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).
- Dialog simpan atau buka file Windows, dijawab oleh `answer-dialog.ps1`.

## Alur

### 1. The card

- the Params tab lists each :name of the path once
- an empty path variable stops the send with a clear message
- the URL preview shows the filled path
- values go into their segments, encoded, and the query keeps its text
- the tab count includes path variables
- renaming a segment updates the table
- a value comes back with its name and only names in the URL are saved

### 2. Data-driven run

- a CSV data file can be chosen
- one pass per row, each titled with its data
- each pass sends its row as variables and path values
- test scripts read pm.iterationData and pm.info
- without a data file, Iterations repeats the run

### 3. Export and import

- export writes url.variable
- import fills path variables from url.variable and adds the missing ones empty
- test data cleaned up

---

Sebelumnya: [Wording, Quick search, dan simpan body response](stage-15.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Auth: Digest, AWS SigV4, dan OAuth 2.0](stage-17.md)
