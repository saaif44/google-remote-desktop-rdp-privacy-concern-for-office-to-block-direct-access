# Installer Notes

UniGlobe Access Guard is configured as a Tauri v2 desktop app for Windows and
macOS.

## Current service-control status

This installer-ready build uses real Chrome Remote Desktop service control on
supported platforms:

- Windows uses `sc.exe` to query/configure/start/stop the `chromoting` service.
- macOS uses `launchctl` to bootstrap/bootout/kickstart
  `org.chromium.chromoting`.

Set `UNIGLOBE_CONTROLLER=mock` to force mock service state for demos or tests.

## Background behavior

- The app creates a tray icon.
- Closing the window hides it instead of quitting.
- The app enables launch-at-login through Tauri's autostart plugin.
- Set `UNIGLOBE_DISABLE_AUTOSTART=1` before launching the app to skip autostart
  registration during development or smoke tests.

## macOS

Build on macOS:

```bash
npm run build:mac
```

Build a universal Apple Silicon + Intel bundle:

```bash
npm run build:mac:universal
```

Expected artifacts:

- `src-tauri/target/release/bundle/macos/UniGlobe Access Guard.app`
- `src-tauri/target/release/bundle/dmg/UniGlobe Access Guard_0.1.0_*.dmg`
- Universal builds are under
  `src-tauri/target/universal-apple-darwin/release/bundle/`

Production distribution should use an Apple Developer ID certificate,
hardened runtime, notarization, and stapling.

## Windows

Build on Windows:

```powershell
npm run build:windows
```

Expected artifacts:

- `src-tauri\target\release\bundle\nsis\*.exe`
- `src-tauri\target\release\bundle\msi\*.msi`

The Windows installer uses WebView2's download bootstrapper. Production
distribution should use an Authenticode code-signing certificate and a timestamp
server.

Chrome Remote Desktop service-control commands may require administrator rights
or an installed privileged helper depending on local machine policy.

## CI

The GitHub Actions workflow in `.github/workflows/build-installers.yml` builds:

- macOS universal `.app` and `.dmg`
- Windows NSIS `.exe` and WiX `.msi`

The workflow sets a short `CARGO_TARGET_DIR` on Windows (`D:\cargo-target`) to
avoid `link.exe` failures caused by long GitHub Actions checkout paths.

Run it manually from GitHub Actions or by pushing a `v*` tag.
