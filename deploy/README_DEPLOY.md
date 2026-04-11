# modelgate - GitHub Actions (SSH) 部署说明

## 需要你在 GitHub 配置的 Secrets

在仓库 `Settings -> Secrets and variables -> Actions` 中添加（建议用 `Environment: production` 管理）：

- `SSH_HOST`: 服务器 IP/域名
- `SSH_PORT`: SSH 端口（常见 `22`）
- `SSH_USER`: SSH 用户名（建议非 root）
- `SSH_PRIVATE_KEY`: 用于登录的私钥（OpenSSH 格式）
- `DEPLOY_ROOT`: 后端部署根目录（例如 `/opt/modelgate`）
- `SERVICE_NAME`: systemd 服务名（例如 `modelgate` 或 `modelgate.service`）
- `DEPLOY_ROOT_FRONTEND`: 前端静态资源根目录，须与服务器一致（例如 `/opt/modelgate/frontend`）；详见 `deploy/frontend/README_FRONTEND_DEPLOY.md`

## 服务器准备（systemd）

以 Ubuntu/Debian 为例：

1) 创建用户与目录

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin modelgate || true
sudo mkdir -p /opt/modelgate
sudo chown -R modelgate:modelgate /opt/modelgate
```

2) 安装 service 文件

- 把仓库里的 `deploy/modelgate.service` 放到 `/etc/systemd/system/modelgate.service`
- 按需修改 `User/Group/WorkingDirectory/ExecStart`

```bash
sudo cp /path/to/modelgate.service /etc/systemd/system/modelgate.service
sudo systemctl daemon-reload
sudo systemctl enable --now modelgate
```

3) 配置敏感环境变量（不要放进仓库）

创建 `/etc/modelgate/modelgate.env`：

```bash
sudo mkdir -p /etc/modelgate
sudo tee /etc/modelgate/modelgate.env >/dev/null <<'EOF'
# 上游 API Key 写在 shared/config.toml 的 [upstream].api_key（勿用 UPSTREAM_API_KEY 环境变量）。
# 可选：仅覆盖上游 base URL（例如 OpenRouter）
# UPSTREAM_BASE_URL=https://openrouter.ai/api/v1
# 可选：覆盖 config 里的 tracing 日志目录（与 [logging].tracing_log_dir 二选一）
# TRACING_LOG_DIR=/opt/modelgate/shared/logs
# 仅直连 OpenAI 时可选：
# OPENAI_ORGANIZATION=...
# OPENAI_PROJECT=...
# RUST_LOG=info
# BYOK：64 位十六进制（32 字节），与 config 中 [byok] master_key_hex 二选一；勿提交到仓库
# BYOK_MASTER_KEY=................................................................
EOF
sudo chmod 600 /etc/modelgate/modelgate.env
```

### 启用 BYOK（控制台「BYOK」与加密存储）

未配置主密钥时，BYOK 相关接口会返回 **503**。任选一种方式配置后 **`sudo systemctl restart modelgate`**（或你的 `SERVICE_NAME`）：

1. **推荐：环境变量**（与 `deploy/modelgate.service` 的 `EnvironmentFile` 一致）  
   在服务器生成密钥并写入 `/etc/modelgate/modelgate.env`（单行、无引号、无多余空格）：

   ```bash
   openssl rand -hex 32
   sudo nano /etc/modelgate/modelgate.env   # 添加一行 BYOK_MASTER_KEY=<上面 64 位 hex>
   sudo systemctl restart modelgate
   ```

2. **或：`shared/config.toml`**（与 CD 使用的配置路径一致）增加：

   ```toml
   [byok]
   master_key_hex = "<64 位十六进制>"
   ```

**注意：** 主密钥一旦用于加密 BYOK 上游密钥后请勿随意更换，否则已存密文将无法解密。轮换需另行迁移数据（当前版本未提供自动迁移）。

部署后确保 **`${DEPLOY_ROOT}/shared/logs`** 存在且 **`modelgate`** 用户可写（CD 工作流会 `mkdir` 并 `chown`）。tracing 日志文件名为 `modelgate.log.YYYY-MM-DD`。

CD 还会在 **`shared/config.toml`** 中写入 **`[sqlite] path = "../../shared/modelgate.db"`**（若尚未配置；相对路径相对 **systemd `WorkingDirectory`**，即 `releases/<sha>`），并把**上一版 release** 里的 `modelgate.db` **复制**到 `shared/`（仅当 `shared/modelgate.db` 尚不存在时），避免每次发版换目录导致**用户/API Key 数据丢失**。

审计日志：**元数据在库里，请求/响应正文在 `[audit].log_dir` 磁盘目录**。CD 会创建 **`shared/audit_logs`**、**`shared/exports`**，并把示例配置里的 `log_dir = "./audit_logs"` / `export_dir = "./exports"` **改写为** `../../shared/audit_logs` 与 `../../shared/exports`（与 SQLite 同级持久化）。若误写过 `../shared/...`（会落到 `releases/shared/`），发版脚本会**合并**到真正的 `shared/` 并修正配置。若 **`shared/audit_logs` 为空** 且上一版 **`current` 指向的 release** 里仍有 `audit_logs/`（或 `exports/`），会**一次性复制**过去，避免首次启用共享路径后历史详情丢失。

### 已有服务器：手工改为共享审计目录（不依赖发版）

若暂时不能跑 CD，可在服务器上执行仓库里的脚本（把审计文件从当前 `current` release 拷到 `shared/`，并改写 `shared/config.toml`）：

```bash
# 在克隆了本仓库的机器上，或把 deploy/migrate-audit-dirs-to-shared.sh 拷到服务器后：
sudo bash deploy/migrate-audit-dirs-to-shared.sh /opt/modelgate
sudo systemctl restart modelgate
```

第二个参数可省略，默认 **`DEPLOY_ROOT=/opt/modelgate`**；若你的 **`DEPLOY_ROOT`** 不同，传入实际路径。

## GitHub Actions 工作流

- `CI`: `.github/workflows/ci.yml`（fmt/clippy/test/build）
- `CD`: `.github/workflows/cd-ssh.yml`（push main 自动 build -> scp -> ssh 解压 -> **删除 /tmp 下本包** -> 切换 current -> systemd restart -> health check）。若服务器 **`/tmp` 为 tmpfs 且很小**，务必保留该清理，否则会解压失败（`No space left on device`）。

## API 子域（Rust 后端对外域名）

生产环境将 Rust 服务暴露在 **`api.modelgate.dev`**（与控制台 `modelgate.dev` 同机时，由 Nginx 反代到本机 Actix 端口，默认 `8000`）。

**DNS：** 为 `api.modelgate.dev` 添加 **A 记录** 指向与 `modelgate.dev` 相同的服务器 IP（例如 `165.22.55.30`）。

**Nginx（在服务器执行一次）：**

```bash
chmod +x ./deploy/api/init-api-nginx.sh
sudo API_DOMAIN=api.modelgate.dev UPSTREAM=http://127.0.0.1:8000 bash ./deploy/api/init-api-nginx.sh
```

前端生产构建通过 `frontend/.env.production` 中的 `VITE_API_BASE_URL` 指向该地址；启用 HTTPS 后改为 `https://api.modelgate.dev` 并同步配置证书。

## 回滚（手动）

服务器上会保留按 commit SHA 命名的目录：

```bash
ls -1 /opt/modelgate/releases
sudo ln -sfn /opt/modelgate/releases/<old_sha> /opt/modelgate/current
sudo systemctl restart modelgate
```

