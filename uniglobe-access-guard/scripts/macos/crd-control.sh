#!/usr/bin/env bash
set -euo pipefail

ACTION="${1:-status}"
MINUTES="${2:-30}"
LABEL="org.chromium.chromoting"
PLIST="/Library/LaunchAgents/org.chromium.chromoting.plist"
UID_VALUE="$(id -u)"
DOMAIN="gui/${UID_VALUE}"
TARGET="${DOMAIN}/${LABEL}"

require_plist() {
  if [ ! -f "$PLIST" ]; then
    echo "Chrome Remote Desktop plist not found at $PLIST" >&2
    exit 1
  fi
}

status() {
  launchctl print "$TARGET" || true
}

enable_start() {
  require_plist
  launchctl enable "$TARGET" || true
  launchctl bootstrap "$DOMAIN" "$PLIST" || true
  launchctl kickstart -k "$TARGET" || true
}

stop_disable() {
  require_plist
  launchctl bootout "$DOMAIN" "$PLIST" || true
  launchctl disable "$TARGET" || true
}

case "$ACTION" in
  status) status ;;
  enable) enable_start ;;
  disable) stop_disable ;;
  start) enable_start ;;
  stop) launchctl bootout "$DOMAIN" "$PLIST" || true ;;
  allow-once)
    echo "Access will start in 60 seconds."
    sleep 60
    enable_start
    sleep "$((MINUTES * 60))"
    stop_disable
    ;;
  *) echo "Usage: $0 status|enable|disable|start|stop|allow-once [minutes]" >&2; exit 1 ;;
esac
