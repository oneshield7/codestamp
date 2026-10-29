# Contributing to Codestamp

Thank you for helping. This guide covers how to set up the project, what a change needs before it is merged, and the rules the code follows.

## Before you start

- For a bug, open an issue with the steps to reproduce it, what you expected, and what happened.
- For a new feature or a larger change, open an issue first so we can agree on the approach before you write the code.
- Never report a security problem in a public issue. See [`SECURITY.md`](SECURITY.md).

## Set up

Requirements and build steps are in the [README](README.md#build-from-source). In short:

```bash
npm install
```

```bash
npm run tauri dev
```

## What a pull request needs

1. `npm run verify` passes with no errors and no warnings. It runs `svelte-check`, ESLint, Prettier, `cargo fmt`, `cargo clippy -D warnings` and the Rust tests.
2. Tests for the change: Rust tests for core logic, and an end-to-end check in [`e2e/`](e2e/README.md) for behavior in the interface. If you change a Copy as code generator, run `node e2e/codegen-e2e.mjs`.
3. Updated documentation when behavior changes: the design notes in [`DESIGN.md`](DESIGN.md) for interface changes, and an entry in [`CHANGELOG.md`](CHANGELOG.md).
4. One topic per pull request, with a description of what changed and why.

## Code rules

Security comes first, then clean code, then reuse.

- **Treat every input as untrusted.** The Rust core validates everything the interface sends again. Use parameterized SQL, pass process arguments as arrays (never a shell string), and never turn off TLS certificate checks.
- **No secrets in code or logs.** Secret values stay encrypted at rest and must never be written to logs, history, exports or Git.
- **Errors are handled.** No empty `catch`. Messages shown to the user are clear and never contain stack traces, queries or file paths.
- **Strict types.** TypeScript runs in strict mode; avoid `any`. Rust passes clippy with warnings as errors.
- **No dead code.** No unused functions, imports or commented-out code, and no leftover debug output.
- **Comments explain why**, not what the code already says.
- **Shared logic lives in one place.** Before adding a helper, check whether one already exists.
- **Names in code are English**, following each language's conventions.

## Interface text

- Write in plain sentence case, and say what happens rather than how good it is.
- Never show keyboard shortcuts in buttons, tooltips, hints or menus. The only list of shortcuts is in Settings › About.
- Every color pair must meet WCAG AA contrast (4.5:1 for text, 3:1 for icons and focus rings), and everything must work from the keyboard.

## License

By contributing, you agree that your contributions are licensed under the [Apache License 2.0](LICENSE).
