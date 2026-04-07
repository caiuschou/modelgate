# modelgate frontend - production initialization and deploy

This guide initializes the frontend production environment and serves the current frontend version on `modelgate.dev`.

## 1) Server initialization (run once on 165.22.55.30)

Run on the server (`DEPLOY_SSH_USER` **must match** GitHub secret `SSH_USER`, or CD cannot write `releases/`):

```bash
chmod +x ./deploy/frontend/init-production.sh
sudo DOMAIN=modelgate.dev DEPLOY_SSH_USER=your_actions_ssh_user bash ./deploy/frontend/init-production.sh
```

### Already ran init without `DEPLOY_SSH_USER`? (Permission denied on CD)

On the server, fix ownership once (replace `your_actions_ssh_user` with the same user as `SSH_USER`):

```bash
sudo chown -R your_actions_ssh_user:www-data /opt/modelgate/frontend
sudo find /opt/modelgate/frontend -type d -exec chmod 2775 {} \;
sudo find /opt/modelgate/frontend -type f -exec chmod 664 {} \;
```

Rust API 子域（与控制台同机时执行一次）：

```bash
chmod +x ./deploy/api/init-api-nginx.sh
sudo API_DOMAIN=api.modelgate.dev UPSTREAM=http://127.0.0.1:8000 bash ./deploy/api/init-api-nginx.sh
```

What the frontend script does:

- Installs Nginx if missing
- Creates deploy directories at `/opt/modelgate/frontend` and sets owner `DEPLOY_SSH_USER:www-data` with setgid on dirs so Actions can write releases and Nginx can read static files
- Enables a **`default_server`** that serves `/var/www/html`（按 **IP** 访问时显示 Nginx 默认页）
- Creates the **域名**站点 `modelgate.dev`（无 `default_server`）与 SPA fallback（`try_files`）
- Restarts Nginx

## 2) GitHub Actions secrets

Configure **environment `production`** secrets (workflow uses `environment: production`):

- `SSH_HOST` = `165.22.55.30`
- `SSH_PORT` = `22`
- `SSH_USER` = same Unix user you set as `DEPLOY_SSH_USER` on the server
- `SSH_PRIVATE_KEY` = private key for SSH login
- `DEPLOY_ROOT_FRONTEND` = `/opt/modelgate/frontend` (absolute path; must exist and be writable by `SSH_USER`)

## 3) CD workflow behavior

Workflow file: `.github/workflows/cd-frontend-ssh.yml`

On push to `main`:

1. Build `frontend/dist`
2. Pack `frontend-${GITHUB_SHA}.tar.gz`
3. Upload package to server
4. Extract to `${DEPLOY_ROOT_FRONTEND}/releases/${GITHUB_SHA}`
5. Point `${DEPLOY_ROOT_FRONTEND}/current` to that release
6. Reload Nginx
7. Health check `http://127.0.0.1/`

## 4) Troubleshooting

**注册 / 登录请求 `POST https://modelgate.dev/api/v1/...` 返回 405**

- 静态站点 Nginx 不接受对 `/api/` 的 POST，除非：  
  - **前端构建**已设置 `VITE_API_BASE_URL=https://api.modelgate.dev`（请求应发往 `api` 子域），或  
  - 已用 `init-production.sh` 里对 `/api/` 的 **反代**（见站点配置），将同机 Actix 暴露在 `127.0.0.1:8000`。
- 重新部署前端（GitHub CD 已在 build 步骤注入 `VITE_API_BASE_URL`），并做一次硬刷新（或清 CDN 缓存）。

若使用 **HTTPS（443）** 的独立 Nginx 配置，请在对应 `server { ... }` 中复制同样的 `location /api/ { ... }` 块。

## 5) Manual rollback

```bash
ls -1 /opt/modelgate/frontend/releases
sudo ln -sfn /opt/modelgate/frontend/releases/<old_sha> /opt/modelgate/frontend/current
sudo nginx -t && sudo systemctl reload nginx
```

