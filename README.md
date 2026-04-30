# Google Remote Desktop / RDP Privacy Concern for Office

This repository contains `UniGlobe Access Guard`, a Tauri desktop app that acts
as a consent wrapper for Chrome Remote Desktop on office computers.

The goal is to block direct remote access while the app is in `Ask to allow`
mode, then bring the app forward when an access request needs local employee
approval.

## App

Source code:

```text
uniglobe-access-guard/
```

Current status:

- Tauri v2, React, TypeScript, Rust.
- Windows and macOS installer configuration.
- Background tray behavior.
- Launch-at-login support.
- Mock service state only.

Important: the current implementation does not run real Chrome Remote Desktop,
`sc.exe`, or `launchctl` service commands. Real service-control adapters should
only be added after the UI and state machine are stable.
