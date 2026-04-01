# modelgate frontend - production initialization and deploy

This guide initializes the frontend production environment and serves the current frontend version on `modelgate.dev`.

## 1) Server initialization (run once on 165.22.55.30)

Run on the server:

```bash
chmod +x ./deploy/frontend/init-production.sh
sudo DOMAIN=modelgate.dev bash ./deploy/frontend/init-production.sh
```

Rust API 子域（与控制台同机时执行一次）：

```bash
chmod +x ./deploy/api/init-api-nginx.sh
sudo API_DOMAIN=api.modelgate.dev UPSTREAM=http://127.0.0.1:8000 bash ./deploy/api/init-api-nginx.sh
```

What the frontend script does:

- Installs Nginx if missing
- Creates deploy directories at `/opt/modelgate/frontend`
- Enables a **`default_server`** that serves `/var/www/html`（按 **IP** 访问时显示 Nginx 默认页）
- Creates the **域名**站点 `modelgate.dev`（无 `default_server`）与 SPA fallback（`try_files`）
- Restarts Nginx

## 2) GitHub Actions secrets

Configure these repository secrets:

- `SSH_HOST` = `165.22.55.30`
- `SSH_PORT` = `22`
- `SSH_USER` = deploy user
- `SSH_PRIVATE_KEY` = private key for SSH login
- `DEPLOY_ROOT_FRONTEND` = `/opt/modelgate/frontend`

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

## 4) Manual rollback

```bash
ls -1 /opt/modelgate/frontend/releases
sudo ln -sfn /opt/modelgate/frontend/releases/<old_sha> /opt/modelgate/frontend/current
sudo nginx -t && sudo systemctl reload nginx
```

