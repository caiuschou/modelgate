#!/usr/bin/env bash
# One-time: point [audit] log_dir / export_dir at ../shared/... and copy existing
# per-release data (same layout as .github/workflows/cd-ssh.yml).
#
# Usage (on the server, as root or with write access to DEPLOY_ROOT):
#   sudo bash /path/to/migrate-audit-dirs-to-shared.sh [/opt/modelgate]
#
# Then: sudo systemctl restart modelgate   # or your SERVICE_NAME

set -euo pipefail

DEPLOY_ROOT="${1:-/opt/modelgate}"
CONFIG="${DEPLOY_ROOT}/shared/config.toml"
SHARED_AUDIT="${DEPLOY_ROOT}/shared/audit_logs"
SHARED_EXPORT="${DEPLOY_ROOT}/shared/exports"
CURRENT_LINK="${DEPLOY_ROOT}/current"

if [[ ! -d "${DEPLOY_ROOT}" ]]; then
  echo "error: DEPLOY_ROOT not found: ${DEPLOY_ROOT}" >&2
  exit 1
fi

mkdir -p "${SHARED_AUDIT}" "${SHARED_EXPORT}"

PREV_CURRENT=""
if [[ -L "${CURRENT_LINK}" ]] || [[ -d "${CURRENT_LINK}" ]]; then
  PREV_CURRENT=$(readlink -f "${CURRENT_LINK}" 2>/dev/null || true)
fi

# Copy body files / exports from the active release before switching paths in config
if [[ -n "${PREV_CURRENT}" ]]; then
  if [[ -d "${PREV_CURRENT}/audit_logs" ]] && [[ -z "$(ls -A "${SHARED_AUDIT}" 2>/dev/null)" ]]; then
    echo "Migrating ${PREV_CURRENT}/audit_logs -> ${SHARED_AUDIT}"
    cp -a "${PREV_CURRENT}/audit_logs/." "${SHARED_AUDIT}/"
  fi
  if [[ -d "${PREV_CURRENT}/exports" ]] && [[ -z "$(ls -A "${SHARED_EXPORT}" 2>/dev/null)" ]]; then
    echo "Migrating ${PREV_CURRENT}/exports -> ${SHARED_EXPORT}"
    cp -a "${PREV_CURRENT}/exports/." "${SHARED_EXPORT}/"
  fi
fi

if [[ ! -f "${CONFIG}" ]]; then
  echo "error: missing ${CONFIG} (create from config.example.toml first)" >&2
  exit 1
fi

sed -i 's/\r$//' "${CONFIG}" || true

if ! grep -q '^\[audit\]' "${CONFIG}"; then
  echo "Appending [audit] with shared paths to ${CONFIG}"
  printf '\n[audit]\nlog_dir = "../shared/audit_logs"\nretention_days = 90\nbatch_size = 50\nflush_interval_seconds = 5\nexport_dir = "../shared/exports"\n' >> "${CONFIG}"
else
  echo "Updating [audit] paths in ${CONFIG}"
  sed -i 's|log_dir = "./audit_logs"|log_dir = "../shared/audit_logs"|g' "${CONFIG}"
  sed -i 's|export_dir = "./exports"|export_dir = "../shared/exports"|g' "${CONFIG}"
fi

if command -v chown >/dev/null 2>&1; then
  chown -R modelgate:modelgate "${SHARED_AUDIT}" "${SHARED_EXPORT}" 2>/dev/null || true
fi

echo "Done. Restart the service, e.g.: sudo systemctl restart modelgate"
echo "Config: ${CONFIG}"
