# CLI: codestamp-cli

Suite [`cli-e2e.mjs`](../../../e2e/cli-e2e.mjs) · 11 checks

`codestamp-cli` runs a Collection v2.1 file with an environment file: variables flow between requests, `--var` fills in auth or reads the process environment, a data file runs once per row, `--bail` stops at the first failure, and JSON and JUnit reports are written. A Collaboration folder can be run too (WebSocket is skipped, streams are cut off). `--help` exits with code 0, a failed request makes the exit code 1, and a run that cannot start exits with code 2 and the reason.

## Requirements

- A built `codestamp-cli.exe`.
- A local HTTP server started by the suite itself (Node `http`).

## Flow

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

Previous: [Basic collection flow](collections.md) · [Stage list](../README.md#stage-list) · Next: [Copy as code: the generated code is run](codegen.md)
