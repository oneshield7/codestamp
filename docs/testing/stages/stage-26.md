# Stage 26: Grup Tools dan Catalog

Suite [`stage26-e2e.mjs`](../../../e2e/stage26-e2e.mjs) · 30 cek

Sidebar berisi Collections dan Tools; halaman lain dicapai lewat status bar dan Quick search. Catalog menilai collection dari History, Run all, dan aturan kualitas: daftar, owner dan tag, overview, runtime, quality, hasil Run all di Tests, tautan flow, dan kolom yang tersisa di jendela sempit. Membuat "E2E Catalog" dan satu flow lalu menghapus keduanya.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).

## Alur

### 1. The sidebar's Tools group

- the sidebar has Tools; the other pages sit in the status bar
- quick search still lists every page, Catalog included
- Environments, Recently deleted and Settings still open from quick search
- Home › Testing has an Catalog tile that opens the catalog

### 2. Fixture: one API with history, rules to break and a flow

Menyiapkan data atau keadaan untuk langkah berikutnya, tanpa cek sendiri.

### 3. The list

- the catalog lists the collection as an API with its error rate and no run yet
- the totals sit above the list
- search narrows the list and an empty result offers to clear the filters
- sorting by error rate puts the highest first and says so

### 4. One API: owner and tags

- the API page has the health chip, no owner yet and four tabs
- a tag with characters that are not allowed is refused with the reason
- owner and tags are saved and shown

### 5. Overview

- the overview splits the score into Runtime, Tests and Quality
- it lists what the API holds and the flows that use it

### 6. Runtime

- Runtime counts 6 sends, 2 errors (33%) and 1 client error from History
- the daily chart stacks errors on answered sends with a legend
- requests are ranked by error rate
- the chart can be read as a table instead

### 7. Quality

- a token typed into a header breaks the secrets rule
- a plain ws:// address and requests without tests are found
- a finding opens its request

### 8. A Run all shows up under Tests

- the catalog comes back to the API that was open
- before any run the Tests tab says so
- after Run all the catalog comes back to Tests with the run kept
- coverage, pass rate and tests come from the run
- each request of the run has its result

### 9. Back to the list: the run, the owner and the tag filter

- the list shows the last run and owner, and filters by tag

### 10. The flow link

- a flow in "Used in Orchestration" opens in Orchestration

### 11. The columns a narrow window keeps

- a narrow list keeps the ranking columns and moves the owner under the name
- the suite ran to the end
- test data cleaned up

---

Sebelumnya: [Title bar dan sidebar yang diciutkan](stage-25.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Indentasi None, SSL/TLS, dan Docs](stage-27.md)
