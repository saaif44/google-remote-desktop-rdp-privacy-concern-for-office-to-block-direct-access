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

## Current status

The app now uses real Chrome Remote Desktop service control on supported desktop
platforms:

- Windows: controls the `chromoting` service with `sc.exe`.
- macOS: controls `org.chromium.chromoting` with `launchctl`.

Set `UNIGLOBE_CONTROLLER=mock` to force the in-memory mock controller for tests
or demos.

Implemented in this phase:

- React dashboard with mode controls, approval actions, countdown modal, settings, and audit log.
- Tauri command API for `get_status`, `set_mode`, `approve_once`,
  `approve_until_revoked`, `terminate_now`, `get_audit_log`, and test incoming requests.
- Rust mock controller, real Windows/macOS controllers, and state-machine tests.
- Background tray behavior: closing the window hides it, while the app keeps running.
- Launch-at-login support for installed desktop builds.
- Test incoming-request flow that brings the window forward and shows a consent modal.
- Installer configuration for macOS `.app`/`.dmg` and Windows NSIS `.exe`/WiX `.msi`.

Important: Chrome Remote Desktop does not expose a public incoming-connection
event API, so the app cannot automatically detect every real incoming request
from CRD itself. The consent modal can be opened by the tray/test request flow
or a future company portal/integration.

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
UNIGLOBE_CONTROLLER=mock cargo test
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
