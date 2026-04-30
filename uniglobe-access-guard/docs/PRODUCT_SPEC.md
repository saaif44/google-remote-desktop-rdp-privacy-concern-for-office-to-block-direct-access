# UniGlobe Access Guard — Product Spec

## 1. Problem

Office machines use Chrome Remote Desktop for support. Staff may have private/personal items open. IT needs access, but the employee must control whether access is allowed.

## 2. Core principle

Do not hide, bypass, or silently observe. The app must give the employee clear control.

## 3. Modes

### Mode A: Allow access all the time

- Chrome Remote Desktop host/service is enabled.
- Service starts automatically after reboot.
- UI status: `Access Always Allowed`.
- Employee can click `Terminate Access Now` at any time.

### Mode B: Ask to allow

- Chrome Remote Desktop host/service is disabled by default.
- Remote device should appear unavailable/offline until employee approves.
- UI status: `Permission Required`.
- Employee can click `Allow Once`.

## 4. Approval flow

1. Employee receives IT request by internal chat/phone/admin portal.
2. Employee opens app or sees an incoming request modal.
3. Employee chooses:
   - `Allow Once for 15 minutes`
   - `Allow Once for 30 minutes`
   - `Allow Until I Revoke`
   - `Deny`
4. If approved:
   - Show warning message.
   - Run 60-second countdown.
   - Enable and start Chrome Remote Desktop host.
   - Start timer.
5. When timer expires:
   - Stop and disable Chrome Remote Desktop host.
   - Write audit log.

## 5. UI screens

### Dashboard

- Header: UniGlobe Access Guard
- Status pill:
  - Green: Access enabled
  - Amber: Waiting for approval
  - Red: Access blocked
- Current mode card:
  - Allow access all the time
  - Ask to allow
- Main buttons:
  - Allow Once
  - Deny / Terminate Now
  - Open Chrome Remote Desktop

### Settings

- Default mode
- Default approval duration: 15/30/60 minutes
- Countdown duration: default 60 seconds
- Admin PIN required to change mode
- Enable/disable startup at login
- Export audit log

### Incoming Request Modal

Text:

`IT is requesting remote access to this office PC. Please save work and close personal/private content before allowing.`

Buttons:

- Allow after 60 seconds
- Deny

## 6. State machine

States:

- `ALWAYS_ALLOWED`
- `ASK_REQUIRED_BLOCKED`
- `PENDING_APPROVAL`
- `COUNTDOWN_ACTIVE`
- `TEMPORARILY_ALLOWED`
- `DENIED`
- `TERMINATED`
- `ERROR`

Transitions:

- `set_always_allowed` -> enable/start CRD
- `set_ask_required` -> stop/disable CRD
- `approve_once` -> countdown -> enable/start CRD -> timer -> stop/disable CRD
- `deny` -> stop/disable CRD
- `terminate_now` -> stop/disable CRD

## 7. Audit events

Write JSONL events:

```json
{"ts":"2026-04-30T12:00:00+06:00","event":"mode_changed","mode":"ASK_REQUIRED_BLOCKED","actor":"local_user"}
{"ts":"2026-04-30T12:01:00+06:00","event":"access_approved","duration_minutes":30,"countdown_seconds":60}
{"ts":"2026-04-30T12:32:00+06:00","event":"access_expired"}
{"ts":"2026-04-30T12:40:00+06:00","event":"access_terminated"}
```

## 8. OS control points

### Windows

Chrome Remote Desktop host service is typically controlled via the Windows service named `chromoting`.

Prototype commands:

```powershell
sc.exe query chromoting
sc.exe config chromoting start= auto
sc.exe start chromoting
sc.exe stop chromoting
sc.exe config chromoting start= disabled
```

### macOS

Chrome Remote Desktop host uses launchd label `org.chromium.chromoting` and a LaunchAgent plist commonly located at:

```text
/Library/LaunchAgents/org.chromium.chromoting.plist
```

Helper files are commonly under:

```text
/Library/PrivilegedHelperTools/org.chromium.chromoting.*
```

Prototype commands:

```bash
launchctl print gui/$(id -u)/org.chromium.chromoting
launchctl bootout gui/$(id -u) /Library/LaunchAgents/org.chromium.chromoting.plist
launchctl bootstrap gui/$(id -u) /Library/LaunchAgents/org.chromium.chromoting.plist
launchctl kickstart -k gui/$(id -u)/org.chromium.chromoting
```

Production should verify the actual installed plist path before running commands.

## 9. Production requirements

- Windows app should be code-signed.
- macOS app should be signed and notarized.
- Privileged helper must require admin approval during installation only.
- Normal employee should not need admin password every time.
- Settings changes should require admin PIN.
- Terminate button should always be available.
