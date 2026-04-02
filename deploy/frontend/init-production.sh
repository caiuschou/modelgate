#!/usr/bin/env bash
set -euo pipefail

# Initialize frontend production environment on Ubuntu/Debian.
# Rust API 使用独立子域，见 deploy/api/init-api-nginx.sh（api.modelgate.dev）。
# Usage:
#   sudo DOMAIN=modelgate.dev DEPLOY_SSH_USER=deploy ./deploy/frontend/init-production.sh
#
# DEPLOY_SSH_USER must match GitHub Actions secret SSH_USER so the CD workflow can
# write releases; www-data (Nginx) stays the group for static file reads.

DOMAIN="${DOMAIN:-modelgate.dev}"
DEPLOY_ROOT="${DEPLOY_ROOT:-/opt/modelgate/frontend}"
DEPLOY_SSH_USER="${DEPLOY_SSH_USER:-}"
NGINX_SITE_PATH="${NGINX_SITE_PATH:-/etc/nginx/sites-available/modelgate-frontend.conf}"

if [[ "${EUID}" -ne 0 ]]; then
  echo "Please run as root (use sudo)." >&2
  exit 1
fi

if ! command -v nginx >/dev/null 2>&1; then
  apt-get update
  apt-get install -y nginx
fi

mkdir -p "${DEPLOY_ROOT}/releases" "${DEPLOY_ROOT}/shared/logs"

if [[ -n "${DEPLOY_SSH_USER}" ]]; then
  chown -R "${DEPLOY_SSH_USER}:www-data" "${DEPLOY_ROOT}"
  find "${DEPLOY_ROOT}" -type d -exec chmod 2775 {} \;
  find "${DEPLOY_ROOT}" -type f -exec chmod 664 {} \; 2>/dev/null || true
else
  chown -R www-data:www-data "${DEPLOY_ROOT}"
  find "${DEPLOY_ROOT}" -type d -exec chmod 755 {} \;
  find "${DEPLOY_ROOT}" -type f -exec chmod 644 {} \; 2>/dev/null || true
  echo "Warning: DEPLOY_SSH_USER is unset. GitHub CD cannot deploy unless SSH_USER is www-data." >&2
  echo "  Re-run with: sudo DEPLOY_SSH_USER=<same as Actions SSH_USER> DOMAIN=${DOMAIN} $0" >&2
fi

# 仅域名访问控制台；按 IP 或未匹配 Host 走 default_server，显示 Nginx 默认页（/var/www/html）
# 文件名用 000- 前缀，保证在 sites-enabled 中先于 modelgate-api / modelgate-frontend 加载，
# 避免在未标记 default_server 的旧配置下第一个 listen 80 变成「隐式默认」从而把 / 代理到后端得到 404。
DEFAULT_CATCHALL="/etc/nginx/sites-available/000-modelgate-default-catchall.conf"
mkdir -p /var/www/html
if [[ ! -f /var/www/html/index.nginx-debian.html && ! -f /var/www/html/index.html ]]; then
  printf '%s\n' '<!DOCTYPE html><html><head><meta charset="utf-8"><title>Welcome to nginx!</title></head><body><h1>Welcome to nginx!</h1></body></html>' \
    > /var/www/html/index.html
fi
cat > "${DEFAULT_CATCHALL}" <<'EOF'
server {
  listen 80 default_server;
  listen [::]:80 default_server;
  server_name _;
  root /var/www/html;
  index index.nginx-debian.html index.html;
}
EOF
rm -f /etc/nginx/sites-enabled/modelgate-default-catchall.conf
ln -sfn "${DEFAULT_CATCHALL}" /etc/nginx/sites-enabled/000-modelgate-default-catchall.conf

cat > "${NGINX_SITE_PATH}" <<EOF
server {
  listen 80;
  listen [::]:80;
  server_name ${DOMAIN} www.${DOMAIN};

  root ${DEPLOY_ROOT}/current;
  index index.html;

  access_log /var/log/nginx/modelgate-frontend-access.log;
  error_log /var/log/nginx/modelgate-frontend-error.log warn;

  location / {
    try_files \$uri /index.html;
  }

  location ~* \.(js|css|png|jpg|jpeg|gif|svg|woff|woff2)$ {
    expires 30d;
    add_header Cache-Control "public, immutable";
  }

  location = /index.html {
    add_header Cache-Control "no-cache, no-store, must-revalidate";
  }
}
EOF

ln -sfn "${NGINX_SITE_PATH}" /etc/nginx/sites-enabled/modelgate-frontend.conf
# 避免与 package 自带 default 重复 default_server
rm -f /etc/nginx/sites-enabled/default
# 清理遗留：曾用裸 server_name _ 把整站反代到后端的单文件（与 default_server 欢迎页冲突）
rm -f /etc/nginx/sites-enabled/modelgate

nginx -t
systemctl enable nginx
systemctl restart nginx

echo "Frontend production environment initialized."
echo "Domain: ${DOMAIN}"
echo "Deploy root: ${DEPLOY_ROOT}"
