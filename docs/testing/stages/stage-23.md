# Stage 23: Orchestration

Suite [`stage23-e2e.mjs`](../../../e2e/stage23-e2e.mjs) · 16 cek

Membangun flow dari Start ke End di atas API toko lokal (register, login, add to cart, checkout) dan memeriksa peta langkahnya: garis nilai yang diterima dan diteruskan langkah terpilih, navigasi dengan panah, checks, flow berhenti (dengan garis berhenti sampai End) atau lanjut saat gagal, urutan dan penyimpanan, request yang terhapus, Quick search, dan menghapus flow. Membuat "E2E Shop" lalu menghapusnya.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).

## Alur

### 1. The page and a new flow

- the sidebar opens the Orchestration page
- a new flow shows Start, Add step and End on the map

### 2. Steps

- the picker offers HTTP requests but not WebSocket ones
- steps are numbered in order on the map, Add step sits before End
- the selected step shows the values it receives and passes on
- arrow keys move between steps on the map

### 3. A passing run

- a run goes from Start to End and every step passes
- values reach later steps: the token and the cart id

### 4. A failing check stops the flow

- a failed step stops the flow; later steps are skipped
- the failed step is selected and says why

### 5. Continue after a failure

- “Continue with the next step” keeps going after a failure

### 6. Reorder, persistence, deleted request

- steps move, and the flow is saved with its rules
- coming back to Orchestration shows a deleted request as a gap

### 7. Quick search and deleting the flow

- quick search finds the Orchestration page
- a flow can be deleted
- test data cleaned up

---

Sebelumnya: [Request gRPC](stage-22.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Body XML dan HTML, SOAP, dan import WSDL](stage-24.md)
