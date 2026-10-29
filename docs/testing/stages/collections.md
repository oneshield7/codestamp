# Collections: Alur collection dasar

Suite [`collections-e2e.mjs`](../../../e2e/collections-e2e.mjs) · 8 cek

Membuat dan mengganti nama collection, membuka request baru sebagai kartu, menunggu autosave, mengirim ke server lokal dan melihat body response yang dirapikan, memastikan baris yang dilipat menyimpan status terakhir, dan memastikan request tetap ada setelah reload. Collection uji dihapus di akhir.

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).

## Alur

### 1. Awal

- collection created and renamed
- new request opens as a card
- autosave finishes
- send returns 200 from local server
- response body shown and formatted
- collapsed row keeps last status
- request persists after reload
- test collection cleaned up

---

Sebelumnya: [Quick search, shortcut, status bar, Export, History, dan Recently deleted](stage-40.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [codestamp-cli](cli.md)
