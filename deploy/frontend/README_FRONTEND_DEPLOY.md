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

**注册 / 登录 `POST https://modelgate.dev/api/v1/...` → 405，且 DevTools 里 Remote Address 是 `172.67.x.x` / `104.21.x.x`**

那是 **Cloudflare** 边缘 IP：浏览器先到 Cloudflare，再由 CF 访问你的源站。**405 几乎总是源站 Nginx** 对路径 `/api/` 没有反代（只有静态 `try_files`），与「挂没挂」无关。

按下面顺序处理：

### A) 源站 443 也要反代 `/api/`（最常见漏配）

`init-production.sh` 只写了 **监听 80** 的站点。若你为 `modelgate.dev` 另有 **HTTPS**（Certbot、Cloudflare Origin Certificate 等），通常会有一个 `listen 443 ssl` 的 `server { ... }`，**里面往往没有** `location /api/`。

在源机执行：

```bash
sudo grep -R "server_name.*modelgate" /etc/nginx/ -n
```

打开对应配置文件，在每个服务于 `modelgate.dev` 的 `server` 块里、**在** `location / { ... }` **之前**插入仓库内片段：

`deploy/frontend/snippets/nginx-api-proxy.conf`

然后：

```bash
sudo nginx -t && sudo systemctl reload nginx
```

再用（本机或笔记本）试：

```bash
curl -i -X POST "https://modelgate.dev/api/v1/auth/register" \
  -H "Content-Type: application/json" \
  -d '{"username":"curl_probe","password":"Xx1_probe_pass","invite_code":"wrong"}'
```

期望：`400` 或 `409` 等 JSON 业务错误；**不应再是 405**。若仍是 405，说明改动的不是实际被 CF 命中的那个 `server` 块。

### B) 让浏览器改请求 `api.modelgate.dev`（推荐长期方案）

确保最新前端 **构建时** 带有 `VITE_API_BASE_URL=https://api.modelgate.dev`（GitHub CD `cd-frontend-ssh.yml` 已注入）。部署后在 Network 里注册请求应指向 **`https://api.modelgate.dev`**，而不是 `modelgate.dev`。

Cloudflare **缓存**：对 `modelgate.dev` 的 JS 做强缓存时，用户可能仍加载旧 bundle。**缓存清除**（Cache Rules / Purge Everything）或版本化文件名未命中时做一次 **硬刷新**。

### C) 仍异常时

在 Cloudflare 控制台看 **Security → Events** 是否拦截；源站 `error.log` 中是否有对应时间的记录。

## 5) Manual rollback

```bash
ls -1 /opt/modelgate/frontend/releases
sudo ln -sfn /opt/modelgate/frontend/releases/<old_sha> /opt/modelgate/frontend/current
sudo nginx -t && sudo systemctl reload nginx
```

