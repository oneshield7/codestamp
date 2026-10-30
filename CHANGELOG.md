# Changelog

All notable changes to Codestamp are listed here. Versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- The installer and uninstaller are in English only, whatever the Windows display language. The language selector and the Indonesian installer text were removed.

### Fixed

- Syncing a team collection no longer fails when teammates' changes need merging on a device whose Git has no global name and email. Codestamp now runs that merge with the name and email set for the collection in Collaboration, as it already did for commits.

## [0.3.6] - 2026-10-29

First public release.

### Added

- Copy as code in 15 languages and tools: cURL, HTTPie, PowerShell, JavaScript fetch and axios, Python requests, PHP Guzzle, Go net/http, Java HttpClient, Kotlin OkHttp, C# HttpClient, Dart http, Swift URLSession, Ruby Net::HTTP and Rust reqwest.
- Copy as code reads `{{name}}` values from environment variables by default (`{{token}}` becomes `TOKEN`), so copied code can be pasted into a repository or a chat without secrets. "As written" and "Filled in (shows secrets)" are still available.
- Copied code follows the same layout in every language: imports, values from the environment, URL and query, headers, body, the request, then the status and body printed.
- An end-to-end suite, `e2e/codegen-e2e.mjs`, that runs the generated code against a local server and checks what the server receives.

### Changed

- The language and value pickers in Copy as code are dropdowns. The last language is remembered on the device; values always start on environment variables.
- Variables named like system environment variables (`{{user}}`, `{{path}}`, `{{home}}`) are read from `USER_VALUE`, `PATH_VALUE` and so on, so the code never picks up the login name or `PATH` by mistake.

## 0.3.5 - 2026-10-29

### Changed

- Orchestration shows a flow as a left-to-right step map. Lines above the map show which values the selected step receives and passes on, and a dashed line shows where a failed run stopped. The selected step's details sit below the map.

## Earlier versions

Versions 0.1.0 to 0.3.4 were built before the repository was public. Their changes are described stage by stage, in Indonesian, in [`README.id.md`](README.id.md).

[Unreleased]: ../../compare/v0.3.6...HEAD
[0.3.6]: ../../releases/tag/v0.3.6
