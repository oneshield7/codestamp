# Changelog

All notable changes to Codestamp are listed here. Versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Linux packages. A `.deb` for Debian 12, Ubuntu 22.04 and later, and the distributions built on them installs Codestamp and `codestamp-cli`. Secret values keep their key in the system keyring (GNOME Keyring or KWallet) through Secret Service.

### Changed

- Codestamp opens even when the system's credential store cannot be reached. Saving a secret then says the store is unavailable, and switching workspaces still works.
- Hints in Environments, Settings and Collaboration name the system Codestamp runs on. The Segoe UI, Cascadia Code and Consolas font choices appear on Windows only.

## [0.4.0] - 2026-11-02

Workspaces, and an environment of its own for each collection. This version updates the database the first time it opens, and older versions cannot open it afterwards. Back up your data first in Settings › Data & history if you may go back.

### Added

- Workspaces. Each workspace keeps its own collections, environments, Globals, flows, cookies and Recently deleted, so work for different teams or clients stays apart. Switch from the button in the middle of the title bar or from Quick search, and manage them in Settings › Workspaces. Settings, Git accounts and certificates are shared. Everything you already had is in "My workspace".
- Move to workspace in the collection header moves a collection, with the environments it owns, to another workspace.
- A workspace's icon can be its first letter, a company logo, an image file or an emoji. Choosing Logo looks for the logo of the websites its name suggests, or of a website you type, through Google's icon service; nothing is sent before you choose Logo. The icon is saved on this device and shows on the launch screen while that workspace opens.
- An HTML response opens as the page itself, with **Page** and **Source** to switch. The page renders in an empty sandbox: Codestamp runs none of its scripts and loads nothing from other sites.
- Resting the pointer on a `{{variable}}` in a request, or moving the caret into it, opens a popover with its value and where it comes from. You can change the value and save it right there.

### Changed

- Each collection keeps its own active environment, so choosing one in a collection no longer changes the others. The picker lists the collection's own environments and the ones kept on this device, and a request never uses another collection's environment. After the update, the environment that was active stays active in every collection that can use it.
- The flow map has one environment picker for each collection its steps use.
- The collection header, with the environment picker and the collection tools, stays at the top while you scroll a long collection.

## [0.3.7] - 2026-10-30

The first release with an installer on GitHub.

### Changed

- The installer and uninstaller are in English only, whatever the Windows display language. The language selector and the Indonesian installer text were removed.

### Fixed

- Syncing a team collection no longer fails when teammates' changes need merging on a device whose Git has no global name and email. Codestamp now runs that merge with the name and email set for the collection in Collaboration, as it already did for commits.

## 0.3.6 - 2026-10-29

The first version with public source code. It has no GitHub release.

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

[Unreleased]: ../../compare/v0.4.0...HEAD
[0.4.0]: ../../releases/tag/v0.4.0
[0.3.7]: ../../releases/tag/v0.3.7
