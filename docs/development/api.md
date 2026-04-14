# ModelGate 服务端 API（当前实现）

**版本:** 1.6  
**更新日期:** 2026年4月9日  
**适用范围:** 本仓库 Rust 服务（`cargo run`）

本文档描述**已实现**的 HTTP 接口。OpenAI 兼容能力的完整产品规格见 [产品 API 文档](../product/api.md)；若与本文冲突，**以本文与 `src/routes.rs` 为准**。

---

## 一、通用约定

### 1.1 Base URL

本地默认：`http://127.0.0.1:8000`（见 `config.toml` 中 `[server]`）。

### 1.2 错误响应

业务错误 JSON 形态（字段以实际代码为准）：

```json
{
  "error": {
    "message": "人类可读说明",
    "type": "validation_error | authentication_error | forbidden_error | conflict_error | not_found_error | rate_limit_error | service_unavailable_error | insufficient_balance | internal_error"
  }
}
```

`insufficient_balance`（402）时 `error` 另含 **`balance_minor`**、**`balance_usd`**、**`usd_scale`**、**`currency`**，见 **3.4**。

### 1.3 CORS

开发环境使用宽松 CORS（`Cors::permissive()`），生产部署请按安全要求收紧。

---

## 二、健康检查

**`GET /healthz`**

- **认证：** 不需要  
- **成功：** `200 OK`

---

## 三、认证与用户

### 3.1 注册

**`POST /api/v1/auth/register`**  
**Content-Type:** `application/json`

```json
{
  "username": "alice",
  "password": "your-password",
  "invite_code": "与服务器配置一致"
}
```

- 邀请码与 `auth.invite_code`（或环境变量 `AUTH_INVITE_CODE`）trim 后**区分大小写**完全一致。  
- 若 `invite_code` 配置为空字符串：自助注册关闭，返回 `400`。  
- **成功：** `201`，`{ "username": "alice" }`  
- **常见错误：** 用户名已存在、邀请码错误、校验失败（`400` / `409`）

### 3.2 登录

**`POST /api/v1/auth/login`**

```json
{
  "username": "alice",
  "password": "your-password"
}
```

- **成功：** `200`，`{ "token": "<api_key>", "user": { "username", "role" } }`  
  - `token` 为数据库中的 API Key（形如 `sk-or-v1-...`），用于后续 `Authorization: Bearer`。  
  - `role`：用户名为 `admin`（不区分大小写）时为 `admin`，否则为 `user`。

### 3.3 当前用户的 API 密钥（控制台）

均需 **`Authorization: Bearer <api_key>`**。

- **未带** `X-Team-Id`：仅列出 / 操作**个人**密钥（`team_id` 为空），归属当前用户。  
- **携带** `X-Team-Id: <numeric_team_id>`：须为该团队成员；列表返回该团队下全部密钥（各成员创建）；新建团队密钥须为 **owner 或 admin**；查看 / 更新 / 吊销规则见代码（创建者可管理自己创建的团队密钥）。

#### 列出密钥（掩码预览）

**`GET /api/v1/me/api-keys`**

可选请求头：`X-Team-Id`（见上）。

**响应示例：**

```json
{
  "data": [
    {
      "id": 1,
      "name": "生产-示例",
      "description": "",
      "preview": "sk-or-v1-12…a3f2",
      "created_at": 1711920000,
      "last_used_at": null,
      "revoked": false,
      "disabled": false,
      "expires_at": null,
      "quota_monthly_tokens": null,
      "quota_used_tokens": 0,
      "max_concurrent_requests": null,
      "quota_monthly_spend_minor": null,
      "quota_used_spend_minor": "0",
      "model_allowlist": null,
      "ip_allowlist": null,
      "status": "active",
      "team_id": null,
      "default_byok_profile_id": null
    }
  ]
}
```

`status`：`active` | `disabled` | `expired` | `revoked`。`team_id`：团队密钥时为团队 id，个人密钥为 `null`。`default_byok_profile_id`：未设置或为 `null` 时，Chat 默认走实例 `[upstream]`；为整数时，未带 `X-MG-Byok-Id` 则走该 BYOK（须与本密钥个人/团队范围一致且未吊销）。

#### 新建密钥

**`POST /api/v1/me/api-keys`**

可选请求头：`X-Team-Id`（owner/admin 可在团队上下文中创建，写入 `team_id`）。

- **Body（JSON，可选）：** 空 body 时等价于 `{ "name": "未命名密钥" }`。  
  - `name`（必填语义）：1–64 字符；未传时使用 `未命名密钥`。  
  - `description`：可选，最长 512 字符。  
  - `expires_at`：可选，Unix 秒。  
  - `quota_monthly_tokens`：可选，正整数，按**自然月**累计 `total_tokens` 用量（仅非流式成功响应计入）。  
  - `max_concurrent_requests`：可选，1–65535；该密钥同时进行的 Chat 上游请求数上限（单实例进程内；多实例不协同）。`0` 或不传表示不限制。  
  - `quota_monthly_spend_minor`：可选，字符串形式的 **USD minor 整数**（与账户余额相同 scale k=15）；按**自然月**累计平台对该密钥的计费扣款（需 `billing.enabled` 且非 BYOK）；达到上限后新请求返回 **`429`**。  
  - `model_allowlist` / `ip_allowlist`：可选，JSON 数组字符串；`chat/completions` 请求将校验模型名与客户端 IP（`X-Forwarded-For` 首选）。  
  - `default_byok_profile_id`：可选，正整数；创建时即可指定 Chat 默认 BYOK（语义同 `PATCH`；须在当前个人/团队范围内可用）。不传则默认为 `null`（走 `[upstream]`）。  
- **成功：** `201`，`{ "id", "api_key": "<完整密钥>", "created_at" }` — **完整 `api_key` 仅此次响应返回**。

#### 密钥详情

**`GET /api/v1/me/api-keys/{key_id}`**

- **成功：** `200`，单条密钥对象（与列表项字段一致）。  
- **失败：** `404`。

#### 更新密钥

**`PATCH /api/v1/me/api-keys/{key_id}`**

- **Body（JSON）：** 至少包含一个字段；未出现的字段不修改。  
  - `name`、`description`、`disabled`  
  - `expires_at`：`null` 表示清除过期时间  
  - `quota_monthly_tokens`：`null` 表示取消配额  
  - `max_concurrent_requests`：`null` 表示不限制并发；设为 **正整数** 启用上限  
  - `quota_monthly_spend_minor`：`null` 表示取消月度消费上限；否则为 **正整数 minor 字符串**  
  - `model_allowlist` / `ip_allowlist`：`null` 表示清除策略  
  - `default_byok_profile_id`：设为 BYOK 的 `id`；`null` 表示清除（恢复默认走 `[upstream]`）。须为**正整数**且该 profile 在当前密钥的归属范围内可用。  
- **成功：** `200`，无 JSON 体  
- **失败：** `400`（无可更新字段或校验失败）、`404`

#### 吊销密钥

**`POST /api/v1/me/api-keys/{key_id}/revoke`**

- **成功：** `200`，无 JSON 体  
- **失败：** `404`（非本人或不存在或已吊销）  
- 若吊销的是当前用于 `Authorization` 的密钥，后续请求将 `401`。

### 3.4 余额与计费（控制台）

需在 `config.toml` 中配置 **`[billing]`**（见 `config.example.toml`）。当 **`billing.enabled = true`** 时：

- 网关 **`POST /v1/chat/completions`** 在余额 ≤ 0 时返回 **`402 Payment Required`**，`error.type` 为 **`insufficient_balance`**；JSON 含 **`balance_minor`**（字符串整数）、**`balance_usd`**（字符串）、**`usd_scale`**（固定 **15**）、`currency: "USD"`（无浮点余额字段）。  
- 成功上游 **2xx** 后，扣费金额取自响应 JSON：**优先 `cost_details.upstream_inference_cost`**；否则对 **`cost_details`** 内数值求和；再否则 **`cost`**。以上字段可在**根对象**或 **`usage`** 内（如 OpenRouter 流式最后一块的 `usage.cost` / `usage.cost_details`）。金额换算为 **scale k=15 的整数 minor** 后从余额扣除；若响应中无任何可解析费用，则扣费 **0** 并记日志。  
- E2E 默认 **`billing.enabled = false`**（见 `e2e/config.toml`），避免测试用户零余额阻塞调用。

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/me/billing/balance` | `{ "balance_minor", "balance_usd", "usd_scale", "currency" }` |
| GET | `/api/v1/me/billing/ledger` | 查询参数：`kind` = `deposit` \| `usage_charge`（可选）、`limit`、`offset` → 每条含 `amount_minor` / `amount_usd`、`balance_after_minor` / `balance_after_usd`（字符串）等 |
| POST | `/api/v1/billing/admin-deposit` | 无控制台会话；**`Authorization: Bearer <billing.admin_deposit_password>`**；body：`{ "username": string, "amount_usd": number }`（目标为控制台用户名）。需 **`billing.admin_deposit_enabled = true`** 且 **`admin_deposit_password` 非空**；否则 **`404`**。密码错误 **`401`**；用户不存在 **`404`**；低于 `min_deposit_cents` 对应金额时 **`400`** |

### 3.5 BYOK 配置（控制台）

需在 `config.toml` 中配置 **`[byok] master_key_hex`**（64 位十六进制 = 32 字节）或环境变量 **`BYOK_MASTER_KEY`**；未配置时本节全部接口返回 **`503`**，且 `POST /v1/chat/completions` 无法使用 BYOK（含 **`X-MG-Byok-Id`** 与网关 Key 上 **`default_byok_profile_id`** 的默认 BYOK）。

认证与 **`X-Team-Id`** 语义与 **3.3** 一致：无团队头为**个人** BYOK；带头为**团队** BYOK（列表全员可见；**创建 / 更新 / 吊销** 团队配置须 **owner 或 admin**）。

上游 `api_key` 使用 **AES-256-GCM** 加密后分栏存储（nonce + 密文）；接口**永不**返回完整明文密钥。

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/me/byok-profiles` | `{ "data": [ { id, name, base_url, api_key_preview, created_at, updated_at, revoked } ] }` |
| POST | `/api/v1/me/byok-profiles` | body：`{ "name"?, "base_url", "api_key" }` → `{ id, created_at }` |
| GET | `/api/v1/me/byok-profiles/{id}` | 单条详情（仍无完整 `api_key`） |
| PATCH | `/api/v1/me/byok-profiles/{id}` | body 至少一项：`name` / `base_url` / `api_key`（轮换） |
| POST | `/api/v1/me/byok-profiles/{id}/revoke` | 吊销后不可再用于转发 |

### 3.6 团队与成员

均需 **`Authorization: Bearer <api_key>`**。

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/teams` | 当前用户所属团队列表 `{ data: [...] }` |
| POST | `/api/v1/teams` | 创建团队，body：`{ "name", "slug" }` → `201` + `team` |
| GET | `/api/v1/teams/{id}` | 详情（成员可见） |
| PATCH | `/api/v1/teams/{id}` | owner/admin 更新 `name` / `slug`（JSON 可选字段） |
| DELETE | `/api/v1/teams/{id}` | 仅 **owner**，级联删除成员与团队密钥等 |
| GET | `/api/v1/teams/{id}/members` | 成员列表 |
| PATCH | `/api/v1/teams/{id}/members/{user_id}` | admin+ 将角色改为 `member` 或 `admin`（不可改 owner） |
| DELETE | `/api/v1/teams/{id}/members/{user_id}` | admin+ 移除成员；唯一 owner 不可被移除 |
| POST | `/api/v1/teams/{id}/invitations` | admin+ 邀请，`{ "invitee_username", "role": "member"|"admin" }`；响应含 **一次性 `token`** |
| DELETE | `/api/v1/teams/{team_id}/invitations/{invitation_id}` | admin+ 撤销待处理邀请 |
| POST | `/api/v1/invitations/accept` | `{ "token" }`，登录用户用户名须与邀请一致 |

---

## 四、用户与 Key 管理（内测/管理接口）

> 当前实现**未**在路由层挂载管理员鉴权，部署到公网前务必通过网络层或反向代理限制访问。

### 4.1 创建用户并生成 Key

**`POST /users`**

```json
{ "username": "bob" }
```

**成功：** `201`，`{ "username", "api_key", "created_at" }`（`created_at` 为 Unix 秒）

### 4.2 为已有用户新增 Key

**`POST /users/{username}/keys`**

**成功：** `201`，`{ "username", "api_key", "created_at" }`

---

## 五、OpenAI 兼容代理

### 5.1 Chat Completions

**`POST /v1/chat/completions`**

- **认证：** `Authorization: Bearer <api_key>`（必填）  
- **Body：** OpenAI Chat Completions 请求体（JSON）  
- **行为：** 默认将请求转发至配置的 `upstream.base_url`，使用 `upstream.api_key` 访问上游。  
- **BYOK 与默认上游：** 须配置 `byok.master_key_hex`（或 `BYOK_MASTER_KEY`）；未配置时，任何需要解密 BYOK 的路径返回 **`503`**。  
  - **解析顺序：** **`X-MG-Use-Platform-Upstream: 1`**（或 `true` / `yes`，不区分大小写）→ **强制**使用实例 `[upstream]`，忽略 Key 默认与 `X-MG-Byok-Id`。  
  - 否则若有 **`X-MG-Byok-Id: <正整数>`**：使用该 BYOK（须与当前网关 Key 归属一致：个人 Key → 本人个人 BYOK；团队 Key → 该团队 BYOK）。非正整数或无法解析 → **`400`**。  
  - 否则若该网关 Key 在控制台设置了 **`default_byok_profile_id`**：使用该 BYOK（同样校验归属与未吊销）。  
  - 否则：使用实例 `[upstream]`。  
- **吊销 BYOK：** 吊销某 profile 后，服务端会将引用该 profile 的 `api_keys.default_byok_profile_id` 清空。  
- **流式：** 支持 `stream: true`（SSE 透传）  
- **预付费：** 若启用 `[billing].enabled`，见 **3.4**（余额不足 **`402`**；扣费与上游 `cost` / `cost_details` 对齐）。  
- **可选请求头：** `X-App-Id` — 写入审计日志的 `app_id`；`X-Thread-Id` — 写入审计日志的 `thread_id`（会话/线程标识）  
- **审计 `metadata`：** 含 `is_byok`（bool）、可选 `byok_profile_id`（int），以及原有 `stream` 等字段。  
- **可选环境变量（转发到上游）：** `OPENAI_ORGANIZATION`、`OPENAI_PROJECT`

---

## 六、审计日志 API

以下接口均需：**`Authorization: Bearer <api_key>`**（与登录返回的 `token` 一致）。

- **未带** `X-Team-Id`：仅 **`user_id` = 当前用户且 `team_id` IS NULL** 的日志（个人上下文）。  
- **携带** `X-Team-Id`**：须为团队成员；返回该 **`team_id`** 下全部日志（团队网关密钥产生的审计）。

详情 `GET /api/v1/logs/request/{request_id}` 按记录归属校验：个人记录匹配 `user_id`，团队记录要求当前用户为该团队成员。

### 6.0 统计聚合（控制台）

**`GET /api/v1/analytics`**

| Query 参数 | 说明 |
|------------|------|
| `start_time` / `end_time` | Unix **秒**；均未传时默认 **近 7 天**；跨度超过 **366 天** 时按结束时间向前截断 |
| `model` / `token_id` / `app_id` / `thread_id` | 与列表接口一致（精确匹配） |

**响应：** `summary`（`total_requests`、`success_requests`、`total_tokens`、`total_cost`、`avg_latency_ms`）、`bucket_seconds`（时间桶秒数：约 1h / 1d / 7d 视跨度而定）、`series[]`（`bucket_start`、`request_count`、`total_tokens`）、`by_model[]`（按请求数降序，最多 30 条；空模型为 `(unknown)`）。

### 6.1 列表

**`GET /api/v1/logs/request`**

| Query 参数 | 说明 |
|------------|------|
| `start_time` / `end_time` | Unix **秒**，过滤 `created_at` |
| `user_id` / `token_id` | 精确匹配（用户常由后端限制） |
| `channel_id` | 精确匹配 |
| `model` | 精确匹配 |
| `status_code` | 精确匹配 |
| `keyword` | 模糊匹配 `request_id` / `error_message` / `model` |
| `app_id` | 精确匹配 |
| `thread_id` | 精确匹配（与请求头 `X-Thread-Id` 写入字段对应） |
| `finish_reason` | 逗号分隔，多值为 **OR** |
| `min_prompt_tokens` / `max_prompt_tokens` / `min_completion_tokens` / `max_completion_tokens` | 区间 |
| `limit` | 默认 `100`，范围 `1..=1000` |
| `offset` | 默认 `0` |

**响应：** `{ "data": [ ... ], "total", "limit", "offset" }`

**`GET /api/v1/logs/threads`**

会话汇总列表（`audit_threads`）：**每行一个 `thread_id`**，Query 参数与 **`GET /api/v1/logs/request` 相同**。返回在时间范围内 **至少有一条** 满足相同筛选条件的请求审计行的会话；排序按 **`last_seen_at` 降序**。

**响应字段（`data[]`）：** `thread_id`、`user_id`、`team_id`（个人为 `null`）、`first_seen_at`、`last_seen_at`、`request_count`（该会话累计请求次数，与维表一致）。

### 6.2 详情

**`GET /api/v1/logs/request/{request_id}`**

### 6.3 发起导出

**`POST /api/v1/logs/export`**  
**Body：**

```json
{
  "start_time": 1711843200,
  "end_time": 1711929600,
  "format": "csv"
}
```

字段均可选；`format` 默认由服务实现决定（常见为 `csv` / `json`）。

**响应：** `{ "export_id", "status", "download_url" }`

### 6.4 导出状态与下载

- **`GET /api/v1/logs/export/{export_id}`** — 状态  
- **`GET /api/v1/logs/export/{export_id}/download`** — 文件下载  

---

## 七、相关源码索引

| 模块 | 路径 |
|------|------|
| 路由注册 | `src/routes.rs` |
| 会话注册/登录 | `src/handlers/session.rs` |
| 用户与 Key | `src/handlers/user.rs` |
| 代理 | `src/handlers/proxy.rs` |
| 审计 HTTP | `src/handlers/audit.rs` |
| 我的 API 密钥 | `src/handlers/api_keys.rs` |
| 审计模型与查询参数 | `src/audit.rs` |

---

**文档结束**
