# Stage 35: Perbaikan Git: clone rusak dan sign-in hilang

Suite [`stage35-e2e.mjs`](../../../e2e/stage35-e2e.mjs) · 9 cek

Sync now membangun ulang clone yang kehilangan folder `.git`, dan collection yang sign-in-nya hilang menawarkan tombol Sign in. `https://auth.test/` diarahkan ke server Git HTTP lokal yang menjawab 401 sampai tes "masuk". Credential helper Git dimatikan untuk salinan uji, jadi tidak ada jendela sign-in yang terbuka di desktop.

## Kebutuhan

- Git for Windows. `https://git.test/` dipetakan ke bare repository lokal di `e2e/.work/git-hosts/` (lihat [harness](../README.md#harness)).
- Server Git HTTP lokal (`git http-backend`) yang dibuat suite.

## Alur

### 1. A clone that lost its .git is rebuilt by Sync now

- the collection is shared
- Sync now rebuilds the lost clone and syncs
- the change saved here is sent and the collaborator's arrives
- no temporary clone is left behind

### 2. A lost sign-in offers Sign in

- a collection shares over the HTTP test host
- a missing sign-in shows sign in with a Sign in button
- Sign in that Git refuses says so
- once signed in, Sign in syncs and the button goes away
- test data cleaned up

---

Sebelumnya: [Pembaruan dalam aplikasi](stage-34.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Git changes dengan riwayat panjang](stage-36.md)
