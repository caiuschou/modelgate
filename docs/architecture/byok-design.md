# BYOK（Bring Your Own Key）设计说明

**版本:** 1.0  
**编写日期:** 2026年4月8日  
**文档类型:** 产品与架构设计（规划）  
**适用范围:** ModelGate 用户或团队自带上游供应商密钥，经网关转发 OpenAI 兼容 API

> **落地状态（2026-04-08）：** 后端已实现 **Phase A 主干** + **Phase B 之「网关 Key 默认 BYOK」**：`byok_profiles` 表、控制台 API `GET/POST/PATCH/... /api/v1/me/byok-profiles`、`POST /v1/chat/completions` 支持 **`X-MG-Use-Platform-Upstream`**（强制平台）、**`X-MG-Byok-Id`**、以及 **`api_keys.default_byok_profile_id`**（未带头时的默认 BYOK）；审计 `metadata` 含 **`is_byok`** 与 **`byok_profile_id`**。依赖 **`[byok] master_key_hex`**（64 hex）或 **`BYOK_MASTER_KEY`**；未配置时 BYOK 管理接口返回 **503**，且不能使用 BYOK 转发。控制台 **`/byok-profiles`** 与 **`/api-keys/:id`** 中默认上游设置已接入。细节以 [实现状态](../implementation-status.md)、[开发 API](../development/api.md) 为准。

---

## 1. 概述

### 1.1 目标

在保留 **ModelGate 用户 API Key**（`Authorization: Bearer`）作为访问网关凭证的前提下，允许 **用户或团队** 在控制台配置自己的 **上游供应商密钥**（OpenAI 兼容服务的 `base_url` + `api_key`）。网关转发 Chat Completions 等请求时，使用 **BYOK 配置** 访问上游，而不是（或作为对）实例 `config.toml` 中全局 `[upstream].api_key` 的替代。

### 1.2 与两类密钥的关系

| 概念 | 说明 |
|------|------|
| **用户 API Key** | 调用 ModelGate 的凭证；语义不变，见 [API 密钥管理](../product/api-key-management.md)。 |
| **实例上游 Key** | 部署方在 `config.toml` / 环境变量中配置；无 BYOK 或显式回退时使用。 |
| **BYOK** | 归属于 **用户或团队** 的上游凭证；解密后仅用于出站请求，**不**回显给客户端。 |

### 1.3 设计原则

- **密钥不落明文：** 数据库仅存密文；解密仅在服务端构建上游请求前的短时内存路径中完成。
- **权限边界清晰：** 仅能使用本人或本团队（与 `X-Team-Id` 一致）名下的 BYOK 配置。
- **可观测：** 审计中标记 `is_byok`，并可关联 `byok_profile_id`（或等价标识）；日志与导出中 **禁止** 写入完整上游密钥。
- **渐进交付：** 分阶段实现，先闭环「创建配置 + 请求头选用 + 审计」，再扩展绑定与策略。

---

## 2. 分阶段范围

### 2.1 Phase A（建议 MVP）

- 控制台（及对应 HTTP API）：按 **用户** 或 **团队** 创建 / 列表 / 更新 / 删除（吊销）BYOK 配置项（名称、`base_url`、供应商 `api_key` 仅提交时传输，列表仅掩码后缀）。
- 网关：`POST /v1/chat/completions`（及后续同类代理路由）支持通过请求头选择 BYOK（见 §5）；校验配置归属与团队上下文。
- 审计：成功路径写入 `is_byok = true`；使用实例上游时 `is_byok = false`。
- 仍沿用现有 **模型白名单、IP 白名单、配额** 等与 Bearer Key 绑定的策略（产品可后续定义 BYOK 是否影响计费字段，见 §8）。

### 2.2 Phase B（部分已落地）

- **用户 API Key** 与默认 BYOK 绑定（`api_keys.default_byok_profile_id`；控制台 PATCH 与 **`/api-keys/:id`** UI）：**已实现**。吊销 BYOK 时清空引用该 profile 的默认绑定。  
- 团队策略：是否允许 member 管理 BYOK、是否强制全队使用 BYOK 等：**未实现**（仍以代码与角色为准）。

### 2.3 Phase C（可选）

- 主密钥与数据密钥托管至 **KMS / Vault**（库内仅存引用 ID）。
- 按模型或路由规则选择不同 BYOK；与多渠道调度方案协同（见 [实现状态](../implementation-status.md) 多渠道项）。

---

## 3. 数据模型（建议）

### 3.1 表：`byok_profiles`（名称可调整）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | UUID 或 INTEGER PK | 配置唯一标识 |
| `owner_type` | enum | `user` \| `team` |
| `owner_id` | int | `users.id` 或 `teams.id` |
| `name` | string | 展示名，如「生产-OpenAI」 |
| `base_url` | string | 上游 OpenAI 兼容根 URL |
| `api_key_ciphertext` | blob | AEAD 密文 |
| `api_key_nonce` / `key_version` | 视算法而定 | 支持加密轮换 |
| `created_at`, `updated_at` | timestamp | |
| `revoked_at` | timestamp nullable | 吊销后不可用于新请求 |

**团队归属校验：** `owner_type = team` 时，仅 **该团队 owner/admin**（或产品约定的角色）可写入；`member` 是否可读列表由 Phase B 策略定义。

**加密：** 使用实例级 **主密钥**（环境变量或受限权限文件，不入库）。算法选用成熟 AEAD（如 AES-GCM、ChaCha20-Poly1305）；主密钥轮换时通过 `key_version` 重加密或双读兼容。

### 3.2 审计扩展（建议）

在现有审计 JSON 或列中可选增加：

- `byok_profile_id`：选用 BYOK 时写入，便于控制台筛选（不含密钥）。
- `is_byok`：与 [审计日志技术方案](audit-log-technical-solution.md)、[产品-审计日志](../product/audit-log.md) 一致。

---

## 4. HTTP API（控制台 / 管理面）

以下路径为 **建议命名**，实现时以 [开发 API](../development/api.md) 为准；均需 **用户登录会话** 或等价鉴权。

| 方法 | 路径（示例） | 说明 |
|------|----------------|------|
| `GET` | `/api/v1/me/byok-profiles` | 列表；支持 `X-Team-Id` 切换团队上下文 |
| `POST` | `/api/v1/me/byok-profiles` | 创建；body 含明文 `api_key`（仅 HTTPS） |
| `GET` | `/api/v1/me/byok-profiles/{id}` | 详情；不含完整 `api_key` |
| `PATCH` | `/api/v1/me/byok-profiles/{id}` | 更新名称、`base_url`、轮换密钥 |
| `POST` | `/api/v1/me/byok-profiles/{id}/revoke` | 吊销 |

**响应：** 创建/轮换成功后，可按产品约定 **一次性** 返回确认信息；日常列表仅 `****` + 末段掩码。

---

## 5. 网关转发语义

### 5.1 选用方式（建议）

- 请求头：`X-MG-Byok-Id: <profile_id>`（与现有 `X-Team-Id` 并列）。
- **不建议** 将上游 `api_key` 放在 Chat Completions JSON body（易进入客户端日志与第三方 SDK 持久化）。

### 5.2 解析优先级（与当前实现一致）

1. 若 **`X-MG-Use-Platform-Upstream: 1`**（或 `true` / `yes`）：**强制**使用实例 `[upstream]`，`is_byok = false`（忽略 Key 默认与 `X-MG-Byok-Id`）。
2. 否则若存在合法的 **`X-MG-Byok-Id`**（正整数），且 profile 归属当前 Bearer 用户/团队（与网关 Key 的 `team_id` 一致）：**使用该 BYOK**，`is_byok = true`。非正整数 → **`400`**。
3. 否则若网关 Key 配置了 **`default_byok_profile_id`**：使用该 BYOK（同样校验归属与未吊销），`is_byok = true`。
4. 否则：使用实例配置 `[upstream]`，`is_byok = false`。

### 5.3 错误语义（建议）

| 场景 | HTTP | 说明 |
|------|------|------|
| Profile 不存在 | 404 | 不明确提示是否存在，防枚举 |
| 无权限 | 403 | 非本人/本团队 |
| 已吊销 | 403 或 410 | 产品择一 |
| 解密失败 | 500 | 记录服务端错误，响应对用户模糊 |

出站请求头：`Authorization: Bearer <解密后的上游 api_key>`，目标 URL 由 profile 的 `base_url` 与路径拼接规则决定（与当前单上游实现一致）。

---

## 6. 安全与合规

- **传输：** 仅 TLS；文档与 UI 明确提示密钥用途与存储方式。
- **存储：** 备份、只读副本、日志管道均按密文同等保护。
- **内存：** 解密结果使用完毕后尽快清零或缩短生命周期（语言级最佳努力）。
- **审计与导出：** 不包含上游密钥；`base_url` 可记录，若担心泄露内网域名可仅记规范化主机名哈希（可选）。
- **计费：** 若产品定义「BYOK 不计平台推理成本」，则 `cost` / `cost_details` 与配额策略需在计费模块单独分支（与 Phase A 可解耦）。

---

## 7. 测试与验收要点

- **单元测试：** 加解密、owner/团队权限矩阵、header 解析顺序。
- **集成测试：** Mock 上游；断言出站 `Authorization` 与 `Host`/`URL` 来自 BYOK。
- **E2E（可选）：** 控制台创建 BYOK → 使用 MG Key + `X-MG-Byok-Id` 发起一次 chat（Playwright 与 [E2E 方案](../development/e2e-testing-plan.md) 一致）。

---

## 8. 开放决策（实现前评审）

1. MVP 是否 **同时** 支持个人与团队 BYOK，或先做其一。  
2. 同一 owner 下是否允许多条 profile（建议允许，便于多供应商）。  
3. BYOK 请求是否仍占用 **月度 Token 配额**（建议仍记录用量，配额策略可配置）。  
4. 主密钥运维方式：环境变量、文件、或后续 KMS。

---

## 9. 相关文档

- [API 密钥管理（产品）](../product/api-key-management.md) — 用户 Key 与上游 Key 术语边界  
- [请求审计日志（产品）](../product/audit-log.md) — `is_byok` 字段说明  
- [审计日志技术方案](audit-log-technical-solution.md) — 扩展字段与存储  
- [实现状态](../implementation-status.md) — 代码真值  
- [产品范围与原则](../product/product-scope-principles.md) — 实例级隔离与团队边界  

---

**文档结束**
