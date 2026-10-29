# Suite end-to-end Codestamp

Skrip di folder ini menjalankan build release Codestamp yang sebenarnya, lalu mengeklik dan mengetik di jendelanya lewat WebView2 DevTools Protocol (port 9333). Setiap suite mencetak satu baris `PASS` atau `FAIL` per cek.

Alur setiap suite, harness, dan cara menjalankannya dijelaskan di [docs/testing](../docs/testing/README.md).

Mulai cepat, dari folder project (Windows):

```bash
npm install --prefix e2e
```

```bash
CARGO_TARGET_DIR="$PWD/e2e/.work/target" npx tauri build --no-bundle
```

```bash
bash e2e/run-list.sh default stage40
```

`codegen-e2e.mjs` tidak butuh aplikasinya. Suite ini menjalankan kode hasil Copy as code di setiap bahasa yang toolchain-nya terpasang:

```bash
node e2e/codegen-e2e.mjs
```

File yang dibuat saat tes berjalan disimpan di `e2e/.work/` dan tidak ikut di-commit. Cadangkan data aplikasi (`%APPDATA%\dev.codestamp.desktop`) sebelum menjalankan suite, karena suite membuat data uji dan mengubah beberapa pengaturan.
