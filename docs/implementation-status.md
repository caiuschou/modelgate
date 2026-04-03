# ModelGate 实现状态（文档与代码对照）

**版本:** 1.0  
**更新日期:** 2026年4月3日  

本文档说明**当前仓库代码已落地的能力**，与 `docs/product/`、`docs/design/` 中描述的**产品目标与交互规格**区分。阅读产品文档时，请同时参考本节，避免将「规划能力」误认为已上线。

---

## 一、后端（Rust / Actix-web）

| 能力 | 状态 | 说明 |
|------|------|------|
| 健康检查 `GET /healthz` | ✅ | |
| 用户注册 `POST /api/v1/auth/register` | ✅ | 用户名 + 密码 + 邀请码；`auth.invite_code` 为空则关闭自助注册 |
| 用户登录 `POST /api/v1/auth/login` | ✅ | 返回 JWT 式用途的 **API Key 字符串**（存于 `api_keys` 表），非 OAuth token |
| 创建用户 `POST /users` | ✅ | 无鉴权（管理/内测用途）；返回新用户与 `api_key` |
| 为用户新增 Key `POST /users/{username}/keys` | ✅ | 无鉴权（管理/内测用途） |
| Chat Completions `POST /v1/chat/completions` | ✅ | Bearer 用户 Key；转发至配置的单一上游 `upstream.base_url`；支持流式 |
| 请求审计日志查询/详情/导出 | ✅ | 见 [开发 API 文档](development/api.md)、[审计日志产品说明](product/audit-log.md) |
| 当前用户 API 密钥 `GET/POST /api/v1/me/api-keys`、`POST .../revoke` | ✅ | 需 Bearer；列表仅掩码预览，创建时返回完整密钥一次 |
| 多渠道配置与路由 | ❌ | 上游为 **一个** `base_url` + `api_key`（环境变量 `UPSTREAM_*`） |
| `/v1/completions`、`/v1/embeddings`、Images、Audio | ❌ | 未注册路由 |
| 用量 API `GET /v1/usage` 等 | ❌ | |
| API 密钥吊销、IP 白名单、配额扣减 | ❌ | DB 有 `revoked` 等字段的部分能力未暴露为完整产品流程 |
| 限流响应头 `X-RateLimit-*` | ❌ | |

**权威路由列表：** `src/routes.rs`。  
**配置项：** `src/config.rs`、`config.example.toml`。

---

## 二、前端（React / Vite 控制台）

| 路由 | 状态 | 说明 |
|------|------|------|
| `/login`、`/register` | ✅ | |
| `/` 首页（仪表盘） | ✅ | 以现有页面为准 |
| `/logs`、`/logs/:requestId` | ✅ | 日志中心 |
| `/api-keys` | ✅ | **API 密钥**列表、新建（完整密钥仅展示一次）、吊销 |
| `/channels`、`/users`、`/analytics`、`/settings` | ⏳ | **占位页**（「页面建设中」）；管理员菜单项部分受 `AdminGuard` 限制 |

**开发代理：** `frontend/vite.config.ts` 将 `/api`、`/healthz`、`/users` 代理到 `http://127.0.0.1:8000`。  
**注意：** `POST /v1/chat/completions` 不在 Vite 代理中，客户端应用需直接请求网关地址（或自行配置反向代理）。

---

## 三、测试与 CI

| 项目 | 说明 |
|------|------|
| 前端 E2E | Playwright，见 [前端 E2E 测试方案](development/e2e-testing-plan.md) |
| GitHub Actions | `.github/workflows/ci-e2e.yml`（变更 `frontend/`、`e2e/`、`src/` 等时触发） |

---

## 四、文档阅读建议

1. **对接接口、部署、排错：** [开发 API](development/api.md)、[部署](development/deployment.md)。  
2. **产品愿景与完整规格：** [产品概述](product/overview.md)、[功能详解](product/features.md)、[API 产品规格](product/api.md)（其中部分接口为未来形态）。  
3. **用户 API Key 管理（产品）：** [API 密钥管理](product/api-key-management.md)。  
4. **审计与日志中心：** [产品-审计日志](product/audit-log.md)、[日志中心交互](design/interaction/log-center.md)。  

---

**文档结束**
