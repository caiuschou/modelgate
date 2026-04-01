#!/usr/bin/env bash
set -euo pipefail

# Nginx reverse proxy for the Rust backend (Actix) on a dedicated API host.
# Usage:
#   sudo API_DOMAIN=api.modelgate.dev UPSTREAM=http://127.0.0.1:8000 ./deploy/api/init-api-nginx.sh

API_DOMAIN="${API_DOMAIN:-api.modelgate.dev}"
UPSTREAM="${UPSTREAM:-http://127.0.0.1:8000}"
NGINX_SITE_PATH="${NGINX_SITE_PATH:-/etc/nginx/sites-available/modelgate-api.conf}"

if [[ "${EUID}" -ne 0 ]]; then
  echo "Please run as root (use sudo)." >&2
  exit 1
fi

if ! command -v nginx >/dev/null 2>&1; then
  apt-get update
  apt-get install -y nginx
fi

cat > "${NGINX_SITE_PATH}" <<EOF
server {
  listen 80;
  listen [::]:80;
  server_name ${API_DOMAIN};

  access_log /var/log/nginx/modelgate-api-access.log;
  error_log /var/log/nginx/modelgate-api-error.log warn;

  location / {
    proxy_pass ${UPSTREAM};
    proxy_http_version 1.1;
    proxy_set_header Host \$host;
    proxy_set_header X-Real-IP \$remote_addr;
    proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto \$scheme;
    proxy_buffering off;
    proxy_read_timeout 3600s;
    proxy_send_timeout 3600s;
  }
}
EOF

ln -sfn "${NGINX_SITE_PATH}" /etc/nginx/sites-enabled/modelgate-api.conf

nginx -t
systemctl enable nginx
systemctl reload nginx

echo "API subdomain Nginx site ready."
echo "Host: ${API_DOMAIN}"
echo "Upstream: ${UPSTREAM}"
