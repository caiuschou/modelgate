#!/usr/bin/env bash
# Grant the deploy / service user permission to read systemd journal (journalctl -u ...).
# Run once on the server as root: sudo bash deploy/grant-deploy-user-journal.sh [username]
# Default username: modelgate
set -euo pipefail

USER_NAME="${1:-modelgate}"

if ! getent group systemd-journal >/dev/null; then
  echo "error: group systemd-journal not found (is this a systemd system?)" >&2
  exit 1
fi

if ! id -u "${USER_NAME}" >/dev/null 2>&1; then
  echo "error: user ${USER_NAME} does not exist" >&2
  exit 1
fi

usermod -aG systemd-journal "${USER_NAME}"

echo "OK: ${USER_NAME} added to group systemd-journal."
echo "The user must open a new login session for the change to apply (log out of SSH and reconnect,"
echo "or use: newgrp systemd-journal)."
echo "Verify: journalctl -u modelgate -b -n 20 --no-pager"
