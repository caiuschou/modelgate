# ModelGate 服务端 API（当前实现）

**版本:** 1.1  
**更新日期:** 2026年4月3日  
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
    "type": "validation_error | authentication_error | conflict_error | not_found_error | internal_error"
  }
}
```

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

均需 **`Authorization: Bearer <api_key>`**，且仅能操作**当前密钥所属用户**名下的记录。

#### 列出密钥（掩码预览）

**`GET /api/v1/me/api-keys`**

**响应示例：**

```json
{
  "data": [
    {
      "id": 1,
      "preview": "sk-or-v1-12…a3f2",
      "created_at": 1711920000,
      "revoked": false
    }
  ]
}
```

#### 新建密钥

**`POST /api/v1/me/api-keys`**

- **Body：** 无  
- **成功：** `201`，`{ "id", "api_key": "<完整密钥>", "created_at" }` — **完整 `api_key` 仅此次响应返回**。

#### 吊销密钥

**`POST /api/v1/me/api-keys/{key_id}/revoke`**

- **成功：** `200`，无 JSON 体  
- **失败：** `404`（非本人或不存在或已吊销）  
- 若吊销的是当前用于 `Authorization` 的密钥，后续请求将 `401`。

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
- **行为：** 将请求转发至配置的 `upstream.base_url` 对应 Chat Completions 路径，使用服务器配置的 `upstream.api_key` 访问上游。  
- **流式：** 支持 `stream: true`（SSE 透传）  
- **可选请求头：** `X-App-Id` — 写入审计日志的 `app_id`  
- **可选环境变量（转发到上游）：** `OPENAI_ORGANIZATION`、`OPENAI_PROJECT`

---

## 六、审计日志 API

以下接口均需：**`Authorization: Bearer <api_key>`**（与登录返回的 `token` 一致）。

普通用户仅能查询**本人** `user_id` 范围内的记录（由服务层过滤）。

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
| `finish_reason` | 逗号分隔，多值为 **OR** |
| `min_prompt_tokens` / `max_prompt_tokens` / `min_completion_tokens` / `max_completion_tokens` | 区间 |
| `limit` | 默认 `100`，范围 `1..=1000` |
| `offset` | 默认 `0` |

**响应：** `{ "data": [ ... ], "total", "limit", "offset" }`

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
