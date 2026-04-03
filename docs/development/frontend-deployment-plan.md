# ModelGate 前端部署方案

**版本:** 1.0  
**制定日期:** 2026年4月1日  
**适用范围:** ModelGate 管理控制台（React + Vite）部署到 staging/prod

> **本地联调：** 开发代理见 `frontend/vite.config.ts`；网关进程见 [部署与运行](deployment.md)。

---

## 一、目标与原则

### 1.1 目标

建立一套稳定、可回滚、可观测的前端部署流程，满足以下要求：

- 支持 staging 与 prod 双环境发布
- 发布过程自动化（CI/CD），降低人工失误
- 故障时可在 5 分钟内完成版本回滚
- 资源缓存策略合理，兼顾性能与更新及时性
- 与后端 API 反向代理无缝协作

### 1.2 部署原则

- **可重复:** 同一版本在不同环境行为一致
- **可追溯:** 每次部署关联 commit、构建号与发布时间
- **可恢复:** 发布失败或异常可快速回滚
- **最小权限:** 服务器与 CI 使用最小必要权限

---

## 二、部署架构

### 2.1 架构说明

采用「静态资源构建 + Nginx 托管 + 反向代理 API」方式：

1. CI 构建前端产物（`dist/`）
2. 将构建包上传到部署机
3. 解压到按 commit SHA 命名的 release 目录
4. 切换 `current` 软链接到新版本
5. reload Nginx 生效

### 2.2 逻辑拓扑

- 用户浏览器 -> Nginx（`modelgate.dev`，静态资源 + SPA）
- 浏览器 -> **`api.modelgate.dev`** -> Nginx 反代 -> Rust（Actix，默认 `127.0.0.1:8000`）
- 生产前端通过 `VITE_API_BASE_URL=http://api.modelgate.dev` 调用后端（见 `frontend/.env.production`）；启用 HTTPS 后改为 `https://api.modelgate.dev`

---

## 三、环境规划

### 3.1 环境划分

| 环境 | 用途 | 域名建议 | 说明 |
|------|------|---------|------|
| dev | 本地开发 | localhost | 本地热更新 |
| staging | 预发布验证 | `staging.modelgate.dev` | 与生产配置尽量一致 |
| prod | 生产环境 | `modelgate.dev` | 对外服务 |

### 3.2 环境变量

建议在构建阶段注入以下变量（以 Vite 为例）：

- `VITE_APP_ENV`：`staging` / `prod`
- `VITE_API_BASE_URL`：后端 API 基址（生产：`http://api.modelgate.dev` 或 HTTPS 等价地址）
- `VITE_SENTRY_DSN`：前端错误监控（可选）
- `VITE_BUILD_SHA`：构建 commit SHA（用于版本追踪）

### 3.3 目标服务器

- 前端部署目标服务器：`165.22.55.30`
- 建议通过 SSH Key 登录并限制来源 IP
- 建议将该地址配置到 CI 的部署目标变量（如 `DEPLOY_HOST=165.22.55.30`）

### 3.4 与后端部署约定对齐

为降低运维复杂度，前端部署复用后端的 SSH 发布约定：

- 部署方式：GitHub Actions + SSH（与后端一致）
- 目录模型：`releases/<commit_sha> + current` 软链接
- 统一 Secrets 命名：`SSH_HOST`、`SSH_PORT`、`SSH_USER`、`SSH_PRIVATE_KEY`、`DEPLOY_ROOT`
- 服务器地址：`SSH_HOST=165.22.55.30`

---

## 四、服务器目录与版本管理

### 4.1 目录结构（推荐）

```text
/opt/modelgate/frontend/
  releases/
    8f9c2d1/
    a1b2c3d/
  current -> /opt/modelgate/frontend/releases/a1b2c3d
  shared/
    logs/
```

### 4.2 版本保留策略

- 生产保留最近 10 个 release
- 预发布保留最近 5 个 release
- 清理旧版本时保留 `current` 与上一个可回滚版本
- release 目录建议使用 `${GITHUB_SHA::7}`（与后端一致，便于追溯）

---

## 五、Nginx 配置规范

### 5.1 核心配置要求

- 启用 HTTPS，HTTP 自动跳转 HTTPS（上线证书后）
- SPA 路由回退到 `index.html`（`try_files $uri /index.html`）
- **Rust API** 使用独立站点 `api.modelgate.dev`（见 `deploy/api/init-api-nginx.sh`），控制台站点可不再配置 `/api/` 反代
- **按 IP 访问**（或未匹配任何 `server_name`）应落在单独的 `default_server`，返回 Nginx 默认页，避免把控制台当作裸 IP 站点暴露
- 对带 hash 的静态资源使用强缓存
- `index.html` 禁止强缓存，确保发布后及时更新

### 5.2 参考配置（IP 默认页 + 控制台域名，简化）

```nginx
# 裸 IP / 未匹配 Host → 默认欢迎页（由 init-production.sh 写入 `000-modelgate-default-catchall.conf`，`000-` 保证先于其他站点加载）
server {
  listen 80 default_server;
  listen [::]:80 default_server;
  server_name _;
  root /var/www/html;
  index index.nginx-debian.html index.html;
}

# 仅 modelgate.dev / www 提供控制台
server {
  listen 80;
  listen [::]:80;
  server_name modelgate.dev www.modelgate.dev;

  root /opt/modelgate/frontend/current;
  index index.html;

  location / {
    try_files $uri /index.html;
  }

  location ~* \.(js|css|png|jpg|jpeg|gif|svg|woff|woff2)$ {
    expires 30d;
    add_header Cache-Control "public, immutable";
  }

  location = /index.html {
    add_header Cache-Control "no-cache, no-store, must-revalidate";
  }
}
```

---

## 六、CI/CD 发布流程

### 6.1 触发策略

- push 到 `main` 后自动部署到 staging
- 手动审批后从 staging 提升到 prod
- hotfix 分支支持手动触发紧急部署

### 6.2 GitHub Secrets（与后端一致）

建议在仓库 `Settings -> Secrets and variables -> Actions` 中配置：

- `SSH_HOST`：`165.22.55.30`
- `SSH_PORT`：SSH 端口（默认 `22`）
- `SSH_USER`：部署用户（建议非 root）
- `SSH_PRIVATE_KEY`：部署私钥（OpenSSH 格式）
- `DEPLOY_ROOT`：前端部署根目录（建议 `/opt/modelgate/frontend`）

> 若直接复用仓库内工作流，前端 CD 使用 `DEPLOY_ROOT_FRONTEND`，见 `.github/workflows/cd-frontend-ssh.yml`。

### 6.3 流程步骤

1. Checkout 代码
2. 安装依赖并执行质量门禁（lint / test / build）
3. 生成构建产物并打包
4. 上传到 `${DEPLOY_ROOT}/releases/${GITHUB_SHA::7}`
5. 切换 `${DEPLOY_ROOT}/current` 软链接
6. `nginx -t` 校验并 `reload`
7. 健康检查（首页 + 关键 API）
8. 记录部署信息（版本、时间、操作者）

### 6.4 质量门禁

- `npm run lint` 无 error
- `npm run test` 通过
- `npm run build` 成功
- 关键路径 E2E（至少登录与日志查询）通过

---

## 七、发布与回滚策略

### 7.1 正常发布流程

1. 合并代码到 `main`
2. 自动部署到 staging，测试通过后审批生产发布
3. 发布完成后执行 smoke test
4. 观察 15~30 分钟关键指标

### 7.2 回滚触发条件

- 首页白屏或主要路由不可访问
- 关键业务功能不可用（登录、日志查询、令牌管理）
- 前端错误率或接口失败率显著升高

### 7.3 回滚步骤

1. 查看可回滚版本：`ls -1 /opt/modelgate/frontend/releases`
2. 将 `current` 指向上一个稳定 release：`ln -sfn /opt/modelgate/frontend/releases/<old_sha> /opt/modelgate/frontend/current`
3. 执行：`nginx -t && systemctl reload nginx`
4. 复测关键路径
5. 记录故障与回滚原因

### 7.4 回滚命令示例

```bash
ls -1 /opt/modelgate/frontend/releases
sudo ln -sfn /opt/modelgate/frontend/releases/<old_sha> /opt/modelgate/frontend/current
sudo nginx -t && sudo systemctl reload nginx
```

---

## 八、安全与可观测性

### 8.1 安全要求

- CI 与服务器仅使用 SSH Key 认证
- 禁止在前端注入任何私密密钥
- 对部署凭据进行周期轮换
- 开启常见安全头（如 `X-Content-Type-Options` 等）

### 8.2 可观测性

- 接入前端错误监控（Sentry 或等价方案）
- 记录版本号与 `build_sha`，支持错误快速定位
- 增加可用性探测（Uptime / Health Check）
- 发布后观察：
  - JS 运行时错误率
  - 关键接口失败率
  - 页面加载性能（LCP）

---

## 九、验收标准

- [ ] staging / prod 发布链路可用
- [ ] 发布全程自动化，具备审批与日志
- [ ] 回滚演练通过（5 分钟内恢复）
- [ ] SPA 路由与 API 代理工作正常
- [ ] 缓存策略符合预期（静态资源强缓存、index 禁缓存）
- [ ] 监控告警可覆盖关键故障场景

---

## 十、实施计划（1 周）

| 天数 | 任务 | 输出 |
|------|------|------|
| Day 1 | Nginx 配置与目录规范落地 | staging 可访问 |
| Day 2 | CI/CD 构建与上传流程打通 | 自动部署到 staging |
| Day 3 | 回滚脚本与发布日志完善 | 可执行回滚 |
| Day 4 | 监控告警接入 | 错误与可用性看板 |
| Day 5 | 生产首发与演练复盘 | 发布报告 + SOP |

---

## 十一、附录：上线检查清单

### 11.1 发布前

- [ ] 版本变更说明已确认
- [ ] 环境变量已校验
- [ ] 数据库变更（如有）已执行
- [ ] staging 回归通过

### 11.2 发布后

- [ ] 首页与核心页面可访问
- [ ] 登录、日志查询、令牌管理功能正常
- [ ] 前端控制台无高优先级报错
- [ ] 错误率与性能指标在阈值内

---

## 十二、落地文件（已提供）

- 服务器初始化脚本：`deploy/frontend/init-production.sh`
- API 子域 Nginx：`deploy/api/init-api-nginx.sh`（`api.modelgate.dev` → 本机 Rust）
- 前端部署说明：`deploy/frontend/README_FRONTEND_DEPLOY.md`
- 前端 CD 工作流：`.github/workflows/cd-frontend-ssh.yml`

---

**文档结束**
