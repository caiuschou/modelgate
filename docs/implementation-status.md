# ModelGate 实现状态（文档与代码对照）

**版本:** 1.5  
**更新日期:** 2026年4月17日  

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
| Chat Completions `POST /v1/chat/completions` | ✅ | Bearer 用户 Key；默认走 `upstream.base_url`；支持 **BYOK**：`X-MG-Use-Platform-Upstream`（强制平台）、`X-MG-Byok-Id`、以及 Key 上 **`default_byok_profile_id`**；支持流式；可按 Key 校验 **模型白名单**、**IP 白名单（CIDR）**、**月度 Token 配额**（非流式成功响应累加） |
| 请求审计日志查询/详情/导出 | ✅ | 见 [开发 API 文档](development/api.md)、[审计日志产品说明](product/audit-log.md)；控制台可选 **`X-Team-Id`** 筛选团队审计；个人上下文仅 `team_id IS NULL` |
| 当前用户 API 密钥 | ✅ | `GET/POST /api/v1/me/api-keys`、`GET/PATCH /api/v1/me/api-keys/{id}`、`POST .../revoke`；名称/描述/禁用/过期/配额/策略、**`default_byok_profile_id`**（默认 BYOK）；`api_key_audit_log` 记录创建/更新/吊销；`last_used_at` 节流更新；可选请求头 **`X-Team-Id`** 切换个人 / 团队密钥上下文（团队下列出全部团队密钥；新建须 owner/admin） |
| 团队与成员 | ✅ | `GET/POST /api/v1/teams`、详情与 PATCH/DELETE；成员列表与角色 PATCH、移除；邀请与接受 `POST /api/v1/invitations/accept`；数据表见迁移 `0007_teams.sql`、`0008_team_scope_api_audit.sql` |
| 多渠道配置与路由 | ❌ | 上游为 **一个** `base_url` + `api_key`（环境变量 `UPSTREAM_*`） |
| 会话级上游亲和（RR + `session_upstream_bindings`） | ✅ | 网关：`POST /v1/chat/completions` 在启用且带会话键时选路；控制台：`PATCH` `session_affinity_enabled` / `upstream_pool`；见 [会话级上游亲和](architecture/session-upstream-affinity.md) |
| BYOK（用户/团队上游密钥） | ✅ | 表 `byok_profiles`、`/api/v1/me/byok-profiles*`；Chat 支持 `X-MG-Use-Platform-Upstream`、`X-MG-Byok-Id`、Key **`default_byok_profile_id`**；吊销 BYOK 时清空引用；审计 `metadata.is_byok`；需配置 `byok.master_key_hex` 或 `BYOK_MASTER_KEY`；规格见 [BYOK 设计](architecture/byok-design.md) |
| `/v1/completions`、`/v1/embeddings`、Images、Audio | ❌ | 未注册路由 |
| 用量 API `GET /v1/usage` 等 | ❌ | |
| API 密钥仅哈希存储（无明文落库） | ❌ | 仍为明文存储，与产品长期安全目标有差距 |
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
| `/api-keys`、`/api-keys/:id` | ✅ | **API 密钥**列表与详情、**新建（模态）**名称/描述/**默认 Chat 上游**、详情页策略与默认上游、禁用/吊销、轮换指引、跳转日志（`token_id` 预填） |
| `/byok-profiles`、`/byok-profiles/:id` | ✅ | **BYOK**：列表/新建/详情/编辑/吊销；随 `X-Team-Id` 个人或团队上下文；未配置服务端主密钥时列表 **503** 提示 |
| `/analytics` | ✅ | 统计页：`GET /api/v1/analytics` 聚合审计日志（时间范围、模型筛选；Recharts 趋势与分布） |
| `/users`、`/settings` | ⏳ | **占位页**（「页面建设中」）；受 `AdminGuard` 限制 |

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
4. **BYOK（规划）：** [BYOK 设计](architecture/byok-design.md)。  
5. **会话级上游亲和（规划）：** [会话级上游亲和](architecture/session-upstream-affinity.md)（含控制台 §7）。  
6. **审计与日志中心：** [产品-审计日志](product/audit-log.md)、[日志中心交互](design/interaction/log-center.md)。  

---

**文档结束**
