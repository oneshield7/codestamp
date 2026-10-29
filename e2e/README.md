# Codestamp end-to-end suites

The scripts in this folder run the real Codestamp release build, then click and type in its window through the WebView2 DevTools Protocol (port 9333). Each suite prints one `PASS` or `FAIL` line per check.

The flow of every suite, the harness, and how to run them are described in [docs/testing](../docs/testing/README.md).

Quick start, from the project folder (Windows):

```bash
npm install --prefix e2e
```

```bash
CARGO_TARGET_DIR="$PWD/e2e/.work/target" npx tauri build --no-bundle
```

```bash
bash e2e/run-list.sh default stage40
```

`codegen-e2e.mjs` does not need the app. It runs the Copy as code output in every language whose toolchain is installed:

```bash
node e2e/codegen-e2e.mjs
```

Files created during a test run are kept in `e2e/.work/` and are not committed. Back up the app data (`%APPDATA%\dev.codestamp.desktop`) before running the suites, because they create test data and change a few settings.
