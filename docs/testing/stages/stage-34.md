# Stage 34: In-app updates

Suite [`stage34-e2e.mjs`](../../../e2e/stage34-e2e.mjs) · 21 checks

Tested against a local update server with a dedicated build. `node updater/prepare.mjs` creates a single-use signing key, `test.conf.json` (the test public key, the address `http://127.0.0.1:9455`, and permission for plain http), a fake "installer" that only records its arguments and is signed for the next version, and `release.json` with the version the suite announces. The version is derived from the app version, so the suite does not need changes for each release. This page also covers `stage34-guard` and `stage34-unset`, which use the normal build.

## Requirements

- `node updater/prepare.mjs`, then the test build with `--config e2e/.work/updater/test.conf.json` (see [Running](../README.md#running)).
- Rust (`rustc`) to compile the fake installer.

## Flow

### 1. The check at start-up

- a newer version shows a banner a few seconds after start
- the check reads latest.json as the Codestamp updater
- the banner offers What's new, Update and restart, and Later

### 2. Release notes in Settings › About

- What's new opens Settings › About with the notes as plain text

### 3. Later

- Later hides the banner but Settings still offers the update

### 4. Unsaved environment edits stop the update

- unsaved environment changes stop the update with a reason

### 5. A tampered download is refused

- a download that fails the signature check is not installed
- the failed update offers Try again

### 6. A release file announcing another version than the signed one

- an installer signed for another version is refused

### 7. Up to date

- the same version reads as up to date and clears the old failure

### 8. Server down

- an unreachable server gives a short reason
- Check for updates is ready again

### 9. Update and restart

- the banner shows download progress
- Codestamp closes and starts the installer in update mode
- the test copy is no longer running

## Unsaved changes guard

Suite [`stage34-guard-e2e.mjs`](../../../e2e/stage34-guard-e2e.mjs).

### 1. Setup

- unsaved collection variables still stop leaving, with the same message
- test data cleaned up

## A build without an update address

Suite [`stage34-unset-e2e.mjs`](../../../e2e/stage34-unset-e2e.mjs).

### 1. Setup

- no update banner without an update server
- the check answers notConfigured without a request
- About says this build is not set up, with no buttons
- the privacy note does not mention an update server

---

Previous: [Status bar, Ctrl+S, manual sync, and shared environments](stage-33.md) · [Stage list](../README.md#stage-list) · Next: [Git fixes: a broken clone and a lost sign-in](stage-35.md)
