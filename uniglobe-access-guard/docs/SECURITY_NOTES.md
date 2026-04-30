# Security Notes

## Do not implement silent access

This product is for consent-based remote support. It must not be used to hide sessions, bypass user awareness, or secretly observe employees.

## Recommended office policy

- No remote access without employee approval except documented emergency/security incidents.
- Employees should use standard accounts, not local admin accounts.
- Admin passwords stay with IT.
- Audit logs should be reviewed weekly.
- Dedicated server/VM machines may use Always Allow.
- Employee-used laptops/desktops should use Ask to Allow.

## Privacy behavior

When `Ask to Allow` is active, Chrome Remote Desktop should be stopped/disabled so remote staff cannot see the screen or hear audio.

## macOS permission limitation

macOS Screen Recording and Accessibility permissions for Chrome Remote Desktop cannot be silently granted by this app. They must be granted by the user/admin according to macOS security prompts.
