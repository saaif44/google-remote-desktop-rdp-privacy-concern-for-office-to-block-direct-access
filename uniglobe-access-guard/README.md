# UniGlobe Access Guard

Cross-platform consent wrapper for Chrome Remote Desktop (Google Remote Desktop) on office machines.

## Product goal

Chrome Remote Desktop stays under a local consent policy:

- **Allow access all the time**: Chrome Remote Desktop host stays enabled.
- **Ask to allow**: Chrome Remote Desktop host stays disabled until the employee explicitly approves access.
- **Allow once**: employee can approve temporary access for a configured duration after a visible countdown.
- **Deny / Terminate**: immediately stops and disables Chrome Remote Desktop.

## Important limitation

Chrome Remote Desktop does not expose a public incoming-request API that this app can intercept. The app works by controlling the OS-level Chrome Remote Desktop host/service.

## Recommended stack

- Tauri v2 + React + TypeScript UI
- Rust backend for local commands and state
- OS-specific privileged helper for production builds
- JSONL audit log

## Phase 1 status

The app currently runs against an in-memory mock Chrome Remote Desktop service state.
No `sc.exe`, `launchctl`, or Chrome Remote Desktop commands are executed by the active
Tauri backend.

Implemented in this phase:

- React dashboard with mode controls, approval actions, countdown modal, settings, and audit log.
- Tauri command API for `get_status`, `set_mode`, `approve_once`,
  `approve_until_revoked`, `terminate_now`, `get_audit_log`, and mock incoming requests.
- Rust mock controller and state-machine tests.
- Background tray behavior: closing the window hides it, while the app keeps running.
- Launch-at-login support for installed desktop builds.
- Mock incoming-request flow that brings the window forward and shows a consent modal.
- Installer configuration for macOS `.app`/`.dmg` and Windows NSIS `.exe`/WiX `.msi`.
- Placeholder Windows/macOS controller structs that intentionally return errors until
  the real service-control phase begins.

## Run and build

Prerequisites:

- Node.js/npm
- Rust/Cargo via rustup

Install dependencies:

```bash
npm install
```

Run the desktop app in development:

```bash
npm run dev
```

Build the web assets and Tauri release binary:

```bash
npm run build
```

Build macOS installers on macOS:

```bash
npm run build:mac
```

Build Windows installers on Windows:

```bash
npm run build:windows
```

Run backend tests:

```bash
cd src-tauri
cargo test
```

## Docs

Read:

- `docs/PRODUCT_SPEC.md`
- `docs/CODEX_BUILD_PROMPT.md`
- `docs/SECURITY_NOTES.md`
- `docs/INSTALLERS.md`

## Scripts

Windows service control:

- `scripts/windows/crd-control.ps1`

macOS launchd control:

- `scripts/macos/crd-control.sh`

These scripts are for Codex/prototyping. Production should use signed privileged helpers.
