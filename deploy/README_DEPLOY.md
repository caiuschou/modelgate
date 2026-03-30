# modelgate - GitHub Actions (SSH) 部署说明

## 需要你在 GitHub 配置的 Secrets

在仓库 `Settings -> Secrets and variables -> Actions` 中添加（建议用 `Environment: production` 管理）：

- `SSH_HOST`: 服务器 IP/域名
- `SSH_PORT`: SSH 端口（常见 `22`）
- `SSH_USER`: SSH 用户名（建议非 root）
- `SSH_PRIVATE_KEY`: 用于登录的私钥（OpenSSH 格式）
- `DEPLOY_ROOT`: 部署根目录（例如 `/opt/modelgate`）
- `SERVICE_NAME`: systemd 服务名（例如 `modelgate` 或 `modelgate.service`）

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
UPSTREAM_API_KEY=xxxxx
# 可选：
# OPENAI_ORGANIZATION=...
# OPENAI_PROJECT=...
EOF
sudo chmod 600 /etc/modelgate/modelgate.env
```

## GitHub Actions 工作流

- `CI`: `.github/workflows/ci.yml`（fmt/clippy/test/build）
- `CD`: `.github/workflows/cd-ssh.yml`（push main 自动 build -> scp -> ssh 解压 -> 切换 current -> systemd restart -> health check）

## 回滚（手动）

服务器上会保留按 commit SHA 命名的目录：

```bash
ls -1 /opt/modelgate/releases
sudo ln -sfn /opt/modelgate/releases/<old_sha> /opt/modelgate/current
sudo systemctl restart modelgate
```

