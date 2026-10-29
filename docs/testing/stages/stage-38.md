# Stage 38: Layar pembuka

Suite [`stage38-e2e.mjs`](../../../e2e/stage38-e2e.mjs) · 9 cek

Setiap reload menjalankan layar pembuka lagi. Untuk Light, Dark, dan Dark Night: tema sudah ada di halaman sebelum kode aplikasi jalan, layar memakai warna latar tema, logo di tengah, layar tampil minimal 0,5 detik, perpindahannya mulus, dan tidak ada transform yang tersisa. Gaya yang disuntikkan saat berjalan (CodeMirror, atribut `style`) tetap berlaku di bawah CSP, dan reduced motion hanya memudarkan aplikasi.

## Alur

### 1. Awal

- styles injected at runtime still apply (CodeMirror, style attributes)
- after launch the screen is gone and the app is left with no transform
- [tema]: the theme is on the page before the app's code runs
- [tema]: the launch screen is drawn in the theme's background
- [tema]: the logo sits in the middle of the window
- [tema]: the app keeps the theme for the next reload
- [tema]: the screen stays at least 0.5 s
- [tema]: the hand-over draws smoothly
- reduced motion: the trace stands still and the app only fades in

---

Sebelumnya: [Grup akses](stage-37.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Posisi jendela, environment aktif, dan lipatan](stage-39.md)
