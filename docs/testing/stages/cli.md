# CLI: codestamp-cli

Suite [`cli-e2e.mjs`](../../../e2e/cli-e2e.mjs) · 11 cek

`codestamp-cli` menjalankan file Collection v2.1 dengan file environment: variabel mengalir antarrequest, `--var` mengisi auth atau membaca environment proses, file data dijalankan sekali per baris, `--bail` berhenti di kegagalan pertama, dan laporan JSON serta JUnit ditulis. Folder Collaboration juga bisa dijalankan (WebSocket dilewati, stream dipotong). `--help` keluar dengan kode 0, request yang gagal membuat kode keluar 1, dan run yang tidak bisa dimulai keluar dengan kode 2 beserta alasannya.

## Kebutuhan

- `codestamp-cli.exe` hasil build.
- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).

## Alur

### 1. A Collection v2.1 file and an environment file

- a failing request makes the run exit 1
- each request and test is printed in notebook order
- variables flow between requests; --var feeds auth
- the summary counts requests and tests
- the JSON report has the app report shape
- the JUnit report lists suites, cases and the failure
- --bail skips what comes after a failure
- a data file runs once per row with its values
- --var <name> reads the process environment

### 2. A Collaboration folder that passes

- a Collaboration folder runs; WebSocket is skipped; a stream is cut short

### 3. The run cannot start

- help exits 0; a run that cannot start exits 2 with the reason

---

Sebelumnya: [Alur collection dasar](collections.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Copy as code: kode yang dihasilkan dijalankan](codegen.md)
