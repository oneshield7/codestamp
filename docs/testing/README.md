# Testing Codestamp

Codestamp is tested from two sides. Static checks and the Rust tests run without a window. The end-to-end (e2e) suites run the real release build, then click and type in the app window through the WebView2 DevTools Protocol (CDP).

This folder describes the flow of every e2e suite: what it sets up, the steps it runs, and the checks that must pass at each step. Each stage has one page in [`stages/`](stages/). The suite scripts and their helpers are in the [`e2e/`](../../e2e/) folder.

## Testing layers

| Layer            | Command             | Contents                                                                               |
| ---------------- | ------------------- | -------------------------------------------------------------------------------------- |
| Types and Svelte | `npm run check`     | `svelte-check` (fails on warnings) and `tsc` for the Node files                        |
| Frontend lint    | `npm run lint`      | ESLint with no warnings, and Prettier                                                  |
| Rust lint        | `npm run lint:rust` | `cargo fmt --check` and `cargo clippy -D warnings`                                     |
| Rust tests       | `npm run test:rust` | 309 unit and integration tests in the Rust core (`src-tauri/src`)                      |
| All of the above | `npm run verify`    | Runs the four above in order                                                           |
| End-to-end       | `e2e/*-e2e.mjs`     | 34 suites, 727 checks against the release build and the Copy as code output, see below |

The Rust tests cover request validation, import and export (Postman, OpenAPI, WSDL, Codestamp files), Git sync and per-field merging, access groups and their encryption, History, Recently deleted, and database migrations. They use an in-memory database, so they never touch app data.

## Harness

The e2e suites run only on Windows, because they use WebView2 and a few PowerShell helpers. Every file created during a test run (the test build, WebView2 profiles, Git bare repositories, screenshots, and updater fixtures) is kept in `e2e/.work/`, which is not committed.

1. **Build a test copy.** `npx tauri build --no-bundle` with `CARGO_TARGET_DIR` pointing to `e2e/.work/target` (an absolute path, because Tauri runs cargo from `src-tauri`), so the test build never clashes with the Codestamp in use from `src-tauri/target`.
2. **`launch.mjs` starts the test copy.** The exe runs with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333` and its own `WEBVIEW2_USER_DATA_FOLDER`. An old test copy is stopped first, and only processes from the test build path are matched, never the user's own Codestamp. `launch.mjs` waits until the collection list has loaded and the launch screen is gone.
3. **Git without a real host.** Through the `GIT_CONFIG_COUNT` variables, the test copy maps `https://git.test/` to bare repositories in `e2e/.work/git-hosts/`. The user's Git configuration is not changed. `E2E_GIT_CONFIG` adds configuration pairs for a single suite (stage35 uses it to turn off the credential helper).
4. **Suites connect over CDP.** A suite takes the page from `http://127.0.0.1:9333/json`, runs JavaScript with `Runtime.evaluate`, then installs small helpers such as `$btn`, `$nav`, and `$row` to find elements. Starting data is created through Tauri commands (`window.__TAURI_INTERNALS__.invoke`), while the steps under test run as clicks, typing, and keyboard events in the real UI.
5. **Results.** Each check prints one `PASS` or `FAIL` line with the observed details, and the suite ends with `N/M passed`. If a suite stops halfway, the lines collected so far are still printed before the error.

Shared helpers:

| File                                                       | Purpose                                                                                      |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `launch.mjs`                                               | Starts the test copy with CDP and waits until it is ready                                    |
| `eval.mjs`                                                 | Runs one expression in the app page and prints the result                                    |
| `run-list.sh`                                              | Runs several suites at one window size, after clearing leftover test data                    |
| `run-sweep.sh`                                             | Runs every suite (except stage25, stage34, and the CLI) at one window size                   |
| `test-process.ps1`                                         | Finds the test copy by its exe path                                                          |
| `answer-dialog.ps1`                                        | Fills in and confirms the Windows open or save file dialog through window messages           |
| `resize-window.ps1`                                        | Resizes the test window to 720×520 or maximizes it                                           |
| `window-rect.ps1`                                          | Reads the window and screen position (stage39)                                               |
| `test-window.ps1`                                          | Finds the test copy's window; used by the other window helpers                               |
| `hit-test.ps1`, `restore-window.ps1`, `desktop-locked.ps1` | Resize edges, restoring the window size, and the locked desktop check (stage25)              |
| `activate-window.ps1`, `menu-keys.ps1`                     | Opens the native menu with the keyboard (stage25)                                            |
| `mcp-server.mjs`                                           | MCP test server over STDIO and Streamable HTTP (stage13)                                     |
| `grpc-shop-server.mjs`, `protos/`                          | gRPC test server with reflection and its `.proto` files (stage22)                            |
| `soap-server.mjs`                                          | SOAP and WSDL test server (stage24)                                                          |
| `access-mate.mjs`                                          | A simulated teammate for access groups, written only from the file format (stage37)          |
| `updater/prepare.mjs`                                      | Creates the test key, the updater build configuration, and a signed fake installer (stage34) |
| `updater/fake-installer.rs`                                | A fake installer that only records its arguments (stage34)                                   |

Other servers the suites need, such as the HTTP echo server, the OAuth server, and the GraphQL server, are started by each suite on the same machine.

## Test data

- Each suite creates its own test data with names that start with "E2E", then deletes it in a `finally` block, also when the suite fails halfway. `run-list.sh` deletes "E2E" collections left over from an interrupted run.
- The stage34 signing key is recreated by `updater/prepare.mjs` on every run and lives only in `e2e/.work/`. Never use the release key for tests.
- Before running e2e, back up `codestamp.db` and its `-wal` and `-shm` files from the app data folder (`%APPDATA%\dev.codestamp.desktop`). Afterwards, restore that backup and compare the hashes. The suites change the theme and a few settings while they run.
- Some suites touch the operating system. stage37 creates a device key in the Windows Credential Manager of the account that runs the tests, and suites that use `answer-dialog.ps1` open the real Windows file dialog.

## Running

Requirements: Windows 10 or 11 with WebView2, Node.js 24 or later, Rust, Git for Windows, and Git Bash for the `.sh` scripts. Run every command below from the project folder.

Install the test server dependencies (gRPC and GraphQL) once:

```bash
npm install --prefix e2e
```

Build the test copy:

```bash
CARGO_TARGET_DIR="$PWD/e2e/.work/target" npx tauri build --no-bundle
```

Start the test copy and run one suite:

```bash
node e2e/launch.mjs
```

```bash
node e2e/stage40-e2e.mjs
```

Run several suites, or all of them, at one window size:

```bash
bash e2e/run-list.sh default stage10 stage12 stage40
```

```bash
bash e2e/run-sweep.sh minimum
```

The CLI e2e uses the debug build of `codestamp-cli`:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --bin codestamp-cli
```

```bash
node e2e/cli-e2e.mjs
```

Copy as code does not need the app. The suite bundles the generators, then runs the generated code with the installed toolchains. A language whose toolchain is missing is marked SKIP:

```bash
node e2e/codegen-e2e.mjs
```

stage34 uses a dedicated updater build. Prepare its fixtures, build, then run the suite. Rebuild the normal test copy afterwards for the other suites:

```bash
node e2e/updater/prepare.mjs
```

```bash
CARGO_TARGET_DIR="$PWD/e2e/.work/target" npx tauri build --no-bundle --config e2e/.work/updater/test.conf.json
```

```bash
node e2e/stage34-e2e.mjs
```

- `run-list.sh <minimum|maximize|default> <suite…>` runs the chosen suites at one window size, starting with a fresh test copy, the Light theme, and no leftover "E2E" collections.
- `run-sweep.sh <minimum|maximize>` runs every suite at one size.
- stage25 runs on its own because it closes the window at the end, and its native menu is tested only with `CODESTAMP_MENU_TEST=1` while the desktop is unlocked.
- stage34 needs the dedicated updater build shown above (see also [its page](stages/stage-34.md)).

## Window sizes

- **Default**: the window as it first opens, with a viewport of about 1280×820.
- **minimum**: 720×520, the smallest supported window size. Below 900px the sidebar becomes the icon rail.
- **maximize**: the monitor resolution.

Some checks run only at certain sizes. For example, the alignment check for the "+" button in the sidebar is skipped when the sidebar is forced into the icon rail, and stage36 checks the side-by-side layout only when the window is wide enough.

## Reading a stage page

Each page holds a summary, what the suite needs, and then its flow. The parts of the flow follow the order of the sections in the suite script. Check names are written exactly as the suite prints them, so they are easy to match with the output. `[theme]` means the check repeats for Light, Dark, and Dark Night. The check count on each page counts the checks written in the suite; a check inside a loop counts once even though it runs several times.

## Adding a stage

1. Copy the structure of the latest suite: the CDP connection, the helpers, `check()`, and the `finally` block that cleans up data.
2. Give test data names that start with "E2E", and create starting data through Tauri commands, not through UI that is not under test.
3. Keep files the suite creates in `e2e/.work/` or in the system temp folder, not in the `e2e/` folder itself.
4. Collection panels (Variables, Auth, Collaboration, Mock server, History, Export) open as a `.panel-dialog` dialog and stay open behind confirmation dialogs. Find other dialogs with `dialog[open]:not(.panel-dialog)`, and find panel content through `section[aria-label="…"]`. A dropdown inside a dialog opens its list only once the dialog has finished appearing, so wait until the options exist instead of using a fixed delay.
5. Add the suite to the list in `run-sweep.sh`, then write its page in this folder.

## Stage list

| Suite                                        | What is tested                                                             | Checks |
| -------------------------------------------- | -------------------------------------------------------------------------- | ------ |
| [stage10-e2e.mjs](stages/stage-10.md)        | Form body, cookie jar, dynamic variables, and Copy as code                 | 37     |
| [stage11-e2e.mjs](stages/stage-11.md)        | Collaboration through Git                                                  | 30     |
| [stage12-e2e.mjs](stages/stage-12.md)        | Basic look, Home, and the New collection dialog                            | 41     |
| [stage13-e2e.mjs](stages/stage-13.md)        | Themes, Settings, colored variables, and the MCP client                    | 28     |
| [stage14-e2e.mjs](stages/stage-14.md)        | Dropdowns and colored icons                                                | 11     |
| [stage15-e2e.mjs](stages/stage-15.md)        | Wording, Quick search, and saving the response body                        | 24     |
| [stage16-e2e.mjs](stages/stage-16.md)        | Path variables and data-driven runs                                        | 15     |
| [stage17-e2e.mjs](stages/stage-17.md)        | Auth: Digest, AWS SigV4, and OAuth 2.0                                     | 15     |
| [stage18-e2e.mjs](stages/stage-18.md)        | OpenAPI and Swagger import                                                 | 9      |
| [stage19-e2e.mjs](stages/stage-19.md)        | GraphQL requests                                                           | 10     |
| [stage20-e2e.mjs](stages/stage-20.md)        | Examples and the mock server                                               | 14     |
| [stage21-e2e.mjs](stages/stage-21.md)        | Server-Sent Events                                                         | 7      |
| [stage22-e2e.mjs](stages/stage-22.md)        | gRPC requests                                                              | 18     |
| [stage23-e2e.mjs](stages/stage-23.md)        | Orchestration                                                              | 16     |
| [stage24-e2e.mjs](stages/stage-24.md)        | XML and HTML bodies, SOAP, and WSDL import                                 | 20     |
| [stage25-e2e.mjs](stages/stage-25.md)        | Title bar and the collapsed sidebar                                        | 16     |
| [stage26-e2e.mjs](stages/stage-26.md)        | The Tools group and Catalog                                                | 30     |
| [stage27-e2e.mjs](stages/stage-27.md)        | None indentation, SSL/TLS, and Docs                                        | 27     |
| [stage28-e2e.mjs](stages/stage-28.md)        | Opening a request and the scroll area                                      | 17     |
| [stage29-e2e.mjs](stages/stage-29.md)        | Find in the editor, new names, and a compact Docs page                     | 23     |
| [stage30-e2e.mjs](stages/stage-30.md)        | A Git account per collection                                               | 26     |
| [stage31-e2e.mjs](stages/stage-31.md)        | Git accounts, Git changes, and replacing a collection                      | 20     |
| [stage32-e2e.mjs](stages/stage-32.md)        | Sharing to a repository that already has files                             | 13     |
| [stage33-e2e.mjs](stages/stage-33.md)        | Status bar, Ctrl+S, manual sync, and shared environments                   | 16     |
| [stage34-e2e.mjs](stages/stage-34.md)        | In-app updates                                                             | 21     |
| [stage35-e2e.mjs](stages/stage-35.md)        | Git fixes: a broken clone and a lost sign-in                               | 9      |
| [stage36-e2e.mjs](stages/stage-36.md)        | Git changes with a long history                                            | 14     |
| [stage37-e2e.mjs](stages/stage-37.md)        | Access groups                                                              | 28     |
| [stage38-e2e.mjs](stages/stage-38.md)        | Launch screen                                                              | 9      |
| [stage39-e2e.mjs](stages/stage-39.md)        | Window position, the active environment, and folding                       | 14     |
| [stage40-e2e.mjs](stages/stage-40.md)        | Quick search, shortcuts, status bar, Export, History, and Recently deleted | 40     |
| [collections-e2e.mjs](stages/collections.md) | Basic collection flow                                                      | 8      |
| [cli-e2e.mjs](stages/cli.md)                 | codestamp-cli                                                              | 11     |
| [codegen-e2e.mjs](stages/codegen.md)         | The Copy as code output run in 15 languages                                | 90     |

Total: 34 suites and 727 checks.
