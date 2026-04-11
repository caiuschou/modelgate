#!/usr/bin/env bash
# One-time (or after user change): align shared config ownership with the GitHub Actions SSH user
# so CD can run sed -i on shared/config.toml.
#
# Usage on server (as root):
#   sudo bash fix-shared-config-owner.sh /opt/modelgate <ssh_username>
#
# <ssh_username> must match the GitHub secret SSH_USER used by cd-ssh.yml.
set -euo pipefail

DEPLOY_ROOT="${1:?usage: $0 DEPLOY_ROOT SSH_USER}"
OWNER_USER="${2:?usage: $0 DEPLOY_ROOT SSH_USER}"
CONFIG="${DEPLOY_ROOT}/shared/config.toml"

if [[ ! -f "${CONFIG}" ]]; then
  echo "error: missing ${CONFIG}" >&2
  exit 1
fi

chown "${OWNER_USER}:${OWNER_USER}" "${CONFIG}"
chmod u+rw "${CONFIG}"
ls -la "${CONFIG}"
echo "ok: ${CONFIG} -> ${OWNER_USER}:${OWNER_USER}"
