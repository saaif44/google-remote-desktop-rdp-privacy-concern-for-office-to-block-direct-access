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
- Real Windows/macOS Chrome Remote Desktop service-control adapters.
- Mock mode remains available with `UNIGLOBE_CONTROLLER=mock`.

Important: Chrome Remote Desktop does not expose a public incoming-connection
event API. The app controls host availability through OS service/launch agent
state and can show a consent modal through the tray/test request flow or a future
company integration.
