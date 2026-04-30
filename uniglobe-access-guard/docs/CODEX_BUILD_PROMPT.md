# Codex Build Prompt

You are building `UniGlobe Access Guard`, a cross-platform desktop app that acts as a consent wrapper for Chrome Remote Desktop.

## Stack

Use:

- Tauri v2
- React
- TypeScript
- Rust commands

## Product behavior

The app has two modes:

1. `Allow access all the time`
   - Start Chrome Remote Desktop host.
   - Set host/service to auto-start after reboot.

2. `Ask to allow`
   - Stop Chrome Remote Desktop host.
   - Disable auto-start.
   - User must approve access before CRD becomes available.

Approval actions:

- `Allow once for 15 minutes`
- `Allow once for 30 minutes`
- `Allow once for 60 minutes`
- `Allow until I revoke`
- `Deny / Terminate now`

When approved:

- Show warning message.
- Run a 60-second countdown.
- Enable/start Chrome Remote Desktop.
- After duration expires, stop/disable it again.

When denied/terminated:

- Stop Chrome Remote Desktop immediately.
- Disable it if current mode is Ask to Allow.

## UI requirements

Create a polished modern GUI:

- Header: UniGlobe Access Guard
- Status badge: Enabled / Permission Required / Countdown / Temporarily Allowed / Error
- Two mode cards:
  - Allow access all the time
  - Ask to allow every time
- Big primary action: Allow Once
- Dangerous action: Deny / Terminate Now
- Countdown modal
- Settings panel
- Audit log viewer
- System tray menu

## Rust command interface

Expose Tauri commands:

```rust
get_status() -> StatusDto
set_mode(mode: Mode) -> Result<StatusDto, String>
approve_once(minutes: u32) -> Result<StatusDto, String>
approve_until_revoked() -> Result<StatusDto, String>
terminate_now() -> Result<StatusDto, String>
get_audit_log() -> Vec<AuditEvent>
```

## Platform adapters

Create a trait:

```rust
trait RemoteDesktopController {
    fn status(&self) -> Result<ServiceStatus, String>;
    fn enable_autostart(&self) -> Result<(), String>;
    fn disable_autostart(&self) -> Result<(), String>;
    fn start(&self) -> Result<(), String>;
    fn stop(&self) -> Result<(), String>;
}
```

Implement:

- `WindowsCrdController`
- `MacCrdController`

### Windows prototype commands

Use `sc.exe`:

```powershell
sc.exe query chromoting
sc.exe config chromoting start= auto
sc.exe config chromoting start= disabled
sc.exe start chromoting
sc.exe stop chromoting
```

Important: before starting a disabled service, set it to `demand` or `auto` first.

### macOS prototype commands

Use launchctl:

```bash
launchctl print gui/$UID/org.chromium.chromoting
launchctl bootout gui/$UID /Library/LaunchAgents/org.chromium.chromoting.plist
launchctl bootstrap gui/$UID /Library/LaunchAgents/org.chromium.chromoting.plist
launchctl kickstart -k gui/$UID/org.chromium.chromoting
```

Verify plist exists before running.

## State files

Windows:

```text
C:\ProgramData\UniGlobeAccessGuard\settings.json
C:\ProgramData\UniGlobeAccessGuard\audit.jsonl
```

macOS:

```text
/Library/Application Support/UniGlobeAccessGuard/settings.json
/Library/Application Support/UniGlobeAccessGuard/audit.jsonl
```

## Safety rules

- Do not bypass user permission.
- Do not hide remote access.
- Do not disable employee’s ability to terminate access.
- Do not delete Chrome Remote Desktop files.
- Stop/disable services only.

## Deliverables

1. Working Tauri project.
2. Clean UI.
3. Service controller abstraction.
4. Windows controller implementation.
5. macOS controller implementation.
6. Installer notes for Windows and macOS.
7. README with install/run/build steps.
8. Tests for state machine logic.

Start by implementing the UI and a mock controller. Then add Windows service control. Then add macOS launchctl control.
