# 代理处理器重构方案

**版本:** 1.0
**编写日期:** 2026年4月1日
**适用范围:** `src/handlers/proxy.rs` 及相关模块的重构

---

## 1. 目的

`src/handlers/proxy.rs` 是 ModelGate 的核心请求转发入口，当前实现在 `chat_completions` 单一函数中承担了认证、请求构建、上游转发、响应处理、审计记录等全部职责。

本方案旨在通过结构化重构提升代码的**可读性、可测试性和可扩展性**，为后续支持多端点（`/v1/embeddings`、`/v1/completions`）和多渠道路由奠定基础。

---

## 2. 现状分析

### 2.1 当前代码结构

```
src/handlers/proxy.rs (202 行)
├── UPSTREAM_HEADERS          — Lazy 静态变量，从环境变量读取
├── chat_completions()        — 主函数，~135 行
├── parse_model_from_request() — serde_json::Value ad-hoc 解析
├── parse_usage_and_cost()    — serde_json::Value ad-hoc 解析
└── send_audit_record()       — 审计消息发送
```

### 2.2 问题清单

| # | 问题 | 影响 | 严重程度 |
|---|------|------|---------|
| P1 | `AuditRecord` 构造出现 3 次，共约 60 行重复代码 | 修改字段时需同步三处，容易遗漏 | 高 |
| P2 | 单函数承担 5 种职责（认证、构建请求、发送、处理响应、审计） | 难以单独测试和复用 | 高 |
| P3 | `UPSTREAM_HEADERS` 用 `once_cell::Lazy` 从环境变量初始化，与 `Config` 体系不一致 | 配置分散，测试困难 | 中 |
| P4 | `parse_model_from_request` / `parse_usage_and_cost` 使用 ad-hoc `Value` 解析 | 缺乏类型安全，字段变更不会编译报错 | 中 |
| P5 | 仅支持 `/v1/chat/completions` 单端点，新端点需复制粘贴整套逻辑 | 扩展性差 | 高 |
| P6 | 流式响应无法提取 usage 信息 | 流式请求审计记录中缺少 token 统计 | 低（后续优化） |

### 2.3 `AuditRecord` 重复示例

当前三处构造仅以下字段不同，其余 ~12 个字段完全一致：

| 场景 | `status_code` | `error_message` | `usage` | `response_body_path` | `metadata` |
|------|:---:|:---:|:---:|:---:|:---:|
| 上游请求失败 | 500 | 固定错误文本 | 无 | 无 | 无 |
| 流式响应 | 实际值 | 无 | 无 | 无 | `{"stream": true}` |
| 非流式响应 | 实际值 | 按状态码判断 | 从响应解析 | 有 | `{"stream": false}` |

---

## 3. 重构方案

### 3.1 AuditRecordBuilder — 消除审计记录构造重复

在 `src/audit.rs` 中新增 builder，将公共字段赋值集中到一处。

**新增结构：**

```rust
pub struct AuditRecordBuilder {
    request_id: String,
    user_id: Option<i64>,
    token_id: Option<i64>,
    channel_id: Option<String>,
    model: Option<String>,
    request_type: String,
    request_body_path: Option<String>,
    start: std::time::Instant,
}

impl AuditRecordBuilder {
    pub fn new(request_id: String, request_type: &str) -> Self {
        Self {
            request_id,
            user_id: None,
            token_id: None,
            channel_id: None,
            model: None,
            request_type: request_type.to_string(),
            request_body_path: None,
            start: std::time::Instant::now(),
        }
    }

    pub fn scope(mut self, token_id: i64, user_id: i64) -> Self {
        self.token_id = Some(token_id);
        self.user_id = Some(user_id);
        self
    }

    pub fn model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }

    pub fn request_body_path(mut self, path: Option<String>) -> Self {
        self.request_body_path = path;
        self
    }

    pub fn build_error(self, error_msg: &str) -> AuditRecord {
        AuditRecord {
            request_id: self.request_id,
            user_id: self.user_id,
            token_id: self.token_id,
            channel_id: self.channel_id,
            model: self.model,
            request_type: Some(self.request_type),
            request_body_path: self.request_body_path,
            response_body_path: None,
            status_code: Some(500),
            error_message: Some(error_msg.to_string()),
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            cost: None,
            latency_ms: Some(self.start.elapsed().as_millis() as i64),
            metadata: None,
            created_at: now_unix_secs(),
        }
    }

    pub fn build_stream(self, status_code: i64) -> AuditRecord {
        AuditRecord {
            status_code: Some(status_code),
            metadata: Some(serde_json::json!({ "stream": true })),
            latency_ms: Some(self.start.elapsed().as_millis() as i64),
            created_at: now_unix_secs(),
            // 公共字段由 builder 填充
            ..self.into_base()
        }
    }

    pub fn build_complete(
        self,
        status_code: i64,
        response_body_path: Option<String>,
        usage: ParsedUsage,
    ) -> AuditRecord {
        AuditRecord {
            status_code: Some(status_code),
            error_message: if status_code >= 400 {
                Some("Upstream returned error status".to_string())
            } else {
                None
            },
            response_body_path,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            cost: usage.cost,
            metadata: Some(serde_json::json!({ "stream": false })),
            latency_ms: Some(self.start.elapsed().as_millis() as i64),
            created_at: now_unix_secs(),
            ..self.into_base()
        }
    }

    fn into_base(self) -> AuditRecord {
        AuditRecord {
            request_id: self.request_id,
            user_id: self.user_id,
            token_id: self.token_id,
            channel_id: self.channel_id,
            model: self.model,
            request_type: Some(self.request_type),
            request_body_path: self.request_body_path,
            response_body_path: None,
            status_code: None,
            error_message: None,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            cost: None,
            latency_ms: None,
            metadata: None,
            created_at: 0,
        }
    }
}
```

**重构后 `proxy.rs` 中的使用方式：**

```rust
let audit = AuditRecordBuilder::new(request_id.clone(), "chat")
    .scope(token_id, user_id)
    .model(parse_model(&body))
    .request_body_path(request_body_path.clone());

// 上游失败
send_audit_record(&state, audit.build_error("Upstream request failed")).await;

// 流式
send_audit_record(&state, audit.build_stream(status_i64)).await;

// 非流式
send_audit_record(&state, audit.build_complete(status_i64, response_body_path, usage)).await;
```

三处构造从 ~60 行缩减为各 1 行。

---

### 3.2 函数职责拆分

将 `chat_completions` 拆分为语义清晰的子函数：

```
chat_completions()                     ← 入口编排，~25 行
  ├── authenticate(&req, &state)       ← 提取 token、校验权限
  ├── build_upstream_request()         ← 构造 reqwest 请求
  ├── handle_stream_response()         ← 流式响应 + 审计
  └── handle_buffered_response()       ← 非流式响应 + 审计
```

**各函数签名：**

```rust
fn authenticate(
    req: &HttpRequest,
    state: &web::Data<AppState>,
) -> Result<(i64, i64), ApiError>;

fn build_upstream_request(
    state: &web::Data<AppState>,
    body: &web::Bytes,
) -> reqwest::RequestBuilder;

async fn handle_stream_response(
    upstream_resp: reqwest::Response,
    status: ActixStatusCode,
    state: &web::Data<AppState>,
    audit: AuditRecordBuilder,
) -> Result<HttpResponse, ApiError>;

async fn handle_buffered_response(
    upstream_resp: reqwest::Response,
    status: ActixStatusCode,
    state: &web::Data<AppState>,
    audit: AuditRecordBuilder,
) -> Result<HttpResponse, ApiError>;
```

**重构后的 `chat_completions` 主函数：**

```rust
pub async fn chat_completions(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Bytes,
) -> Result<HttpResponse, ApiError> {
    let request_id = crate::audit::generate_request_id();
    let (token_id, user_id) = authenticate(&req, &state)?;

    let is_stream = upstream::is_stream_request(&body);
    let request_body_path =
        crate::audit::save_body_to_file(&state.audit_config, &request_id, "request", &body).ok();

    let audit = AuditRecordBuilder::new(request_id, "chat")
        .scope(token_id, user_id)
        .model(parse_model(&body))
        .request_body_path(request_body_path);

    let upstream_resp = match build_upstream_request(&state, &body).send().await {
        Ok(resp) => resp,
        Err(e) => {
            error!(error = %e, "upstream request failed");
            send_audit_record(&state, audit.build_error("Upstream request failed")).await;
            return Err(ApiError::InternalError("Upstream request failed".into()));
        }
    };

    let status = ActixStatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(ActixStatusCode::BAD_GATEWAY);

    if is_stream {
        handle_stream_response(upstream_resp, status, &state, audit).await
    } else {
        handle_buffered_response(upstream_resp, status, &state, audit).await
    }
}
```

---

### 3.3 Upstream Headers 纳入配置体系

**现状：** 模块级 `Lazy<HeaderMap>` 在首次访问时读取 `OPENAI_ORGANIZATION` / `OPENAI_PROJECT` 环境变量。

**方案：** 将上游请求相关配置统一到 `AppState`，在服务启动时构建一次。

**在 `src/upstream.rs` 新增：**

```rust
pub struct UpstreamClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    default_headers: reqwest::header::HeaderMap,
}

impl UpstreamClient {
    pub fn from_config(cfg: &UpstreamConfig, http: reqwest::Client) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        if let Some(ref org) = cfg.organization {
            if let Ok(v) = org.parse() {
                headers.insert("openai-organization", v);
            }
        }
        if let Some(ref project) = cfg.project {
            if let Ok(v) = project.parse() {
                headers.insert("openai-project", v);
            }
        }
        Self { http, base_url: cfg.base_url.clone(), api_key: cfg.api_key.clone(), default_headers: headers }
    }

    pub fn post_chat_completions(&self, body: bytes::Bytes) -> reqwest::RequestBuilder {
        self.http
            .post(build_chat_completions_url(&self.base_url))
            .headers(self.default_headers.clone())
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .body(body)
    }
}
```

**`AppState` 变更：**

```rust
pub struct AppState {
    pub cfg: AppConfig,
    // pub http: reqwest::Client,        ← 移除
    pub upstream: UpstreamClient,         // ← 替代
    pub db: DbConn,
    pub auth_service: Arc<dyn AuthService>,
    pub audit_service: Arc<dyn AuditService>,
    pub user_service: Arc<dyn UserService>,
    pub audit_sender: mpsc::Sender<AuditMessage>,
    pub audit_config: AuditConfig,
}
```

**`config.toml` 扩展：**

```toml
[upstream]
base_url = "https://api.openai.com"
api_key = "sk-..."
organization = "org-..."   # 原 OPENAI_ORGANIZATION 环境变量
project = "proj-..."       # 原 OPENAI_PROJECT 环境变量
```

**影响：** `once_cell` 依赖在此文件中不再需要，`proxy.rs` 中 `UPSTREAM_HEADERS` 静态变量可删除。

---

### 3.4 类型化请求/响应解析

用 `#[derive(Deserialize)]` 结构体替代 ad-hoc `serde_json::Value` 解析。

**新增到 `src/upstream.rs`：**

```rust
#[derive(Deserialize)]
pub struct ChatRequestPartial {
    pub model: Option<String>,
    pub stream: Option<bool>,
}

#[derive(Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
}

#[derive(Deserialize)]
pub struct ChatResponsePartial {
    pub usage: Option<UsageInfo>,
    pub cost: Option<f64>,
}

#[derive(Debug, Default)]
pub struct ParsedUsage {
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cost: Option<f64>,
}

impl From<ChatResponsePartial> for ParsedUsage {
    fn from(resp: ChatResponsePartial) -> Self {
        let (pt, ct, tt) = resp.usage.map_or((None, None, None), |u| {
            (u.prompt_tokens, u.completion_tokens, u.total_tokens)
        });
        Self { prompt_tokens: pt, completion_tokens: ct, total_tokens: tt, cost: resp.cost }
    }
}

pub fn parse_model(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<ChatRequestPartial>(body)
        .ok()
        .and_then(|r| r.model)
}

pub fn parse_usage(body: &[u8]) -> ParsedUsage {
    serde_json::from_slice::<ChatResponsePartial>(body)
        .map(ParsedUsage::from)
        .unwrap_or_default()
}
```

**`proxy.rs` 中的变化：** 删除 `parse_model_from_request` 和 `parse_usage_and_cost`，改为调用 `upstream::parse_model` 和 `upstream::parse_usage`。

---

### 3.5 泛化代理模式（面向未来扩展）

当需要支持多个 OpenAI 兼容端点时，引入端点枚举和通用代理函数。

```rust
pub enum ProxyEndpoint {
    ChatCompletions,
    Embeddings,
    Completions,
}

impl ProxyEndpoint {
    pub fn request_type(&self) -> &str {
        match self {
            Self::ChatCompletions => "chat",
            Self::Embeddings => "embedding",
            Self::Completions => "completion",
        }
    }
}
```

各路由 handler 变为 thin wrapper：

```rust
pub async fn chat_completions(req: HttpRequest, state: web::Data<AppState>, body: web::Bytes)
    -> Result<HttpResponse, ApiError> {
    proxy_request(ProxyEndpoint::ChatCompletions, req, state, body).await
}

pub async fn embeddings(req: HttpRequest, state: web::Data<AppState>, body: web::Bytes)
    -> Result<HttpResponse, ApiError> {
    proxy_request(ProxyEndpoint::Embeddings, req, state, body).await
}
```

> 此步骤建议在实际需要第二个端点时再实施，避免过度设计。

---

## 4. 文件变更汇总

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/audit.rs` | 新增 | `AuditRecordBuilder` 结构体及其方法 |
| `src/upstream.rs` | 新增 + 修改 | `UpstreamClient`、`ChatRequestPartial`、`UsageInfo`、`ParsedUsage`、`parse_model`、`parse_usage` |
| `src/handlers/proxy.rs` | 重写 | 拆分子函数，删除重复代码、静态变量、ad-hoc 解析 |
| `src/main.rs` | 修改 | `AppState` 字段变更：`http` → `upstream: UpstreamClient` |
| `src/config.rs` | 修改 | `UpstreamConfig` 新增 `organization`、`project` 可选字段 |

### 重构前后代码量对比

| 模块 | 重构前 | 重构后 | 说明 |
|------|-------|-------|------|
| `handlers/proxy.rs` | 202 行 | ~70 行 | 职责拆分 + 消除重复 |
| `audit.rs` | 212 行 | ~290 行 | 新增 AuditRecordBuilder (~80 行) |
| `upstream.rs` | 71 行 | ~150 行 | 新增 UpstreamClient + 类型化解析 |
| **合计** | 485 行 | ~510 行 | 总量持平，结构大幅改善 |

---

## 5. 实施计划

### 5.1 分步实施顺序

重构分 3 个阶段，每阶段独立可交付、可验证：

**阶段一（P0）：消除重复 + 拆分函数**

| 步骤 | 内容 | 预计改动 |
|------|------|---------|
| 1a | 在 `audit.rs` 中实现 `AuditRecordBuilder` | 新增 ~80 行 |
| 1b | 在 `proxy.rs` 中拆分子函数并使用 builder | 重写 ~130 行 |
| 1c | 验证编译 + 手动测试 `/v1/chat/completions` | — |

**阶段二（P1）：配置归一化 + 类型安全**

| 步骤 | 内容 | 预计改动 |
|------|------|---------|
| 2a | `config.rs` 新增 `organization`、`project` 字段 | 修改 ~5 行 |
| 2b | `upstream.rs` 实现 `UpstreamClient` | 新增 ~50 行 |
| 2c | `upstream.rs` 新增类型化解析结构体 | 新增 ~40 行 |
| 2d | `main.rs` 中 `AppState` 使用 `UpstreamClient` | 修改 ~10 行 |
| 2e | `proxy.rs` 切换为新接口 | 修改 ~15 行 |

**阶段三（P2）：泛化多端点支持**

| 步骤 | 内容 | 预计改动 |
|------|------|---------|
| 3a | 引入 `ProxyEndpoint` 枚举 + `proxy_request` 通用函数 | 新增 ~30 行 |
| 3b | 新增 `/v1/embeddings` 端点 | 新增 ~10 行 |
| 3c | `routes.rs` 注册新路由 | 修改 ~5 行 |

> 阶段三建议在产品实际需要新端点时再实施。

### 5.2 验证方式

每阶段完成后执行以下验证：

1. `cargo build` — 编译通过
2. `cargo test` — 现有测试通过
3. 手动测试 `POST /v1/chat/completions`（非流式 + 流式）
4. 检查审计日志数据库中记录完整性
5. 检查 `audit_logs/` 目录下请求/响应体文件正常生成

### 5.3 风险评估

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| `AuditRecordBuilder` 的 `..self.into_base()` 语法需要 `AuditRecord` 无自定义 `Drop` | 低 | 编译失败 | 当前 `AuditRecord` 为 `#[derive(Clone)]`，满足要求 |
| `UpstreamClient` 替换 `http` 字段可能影响其他使用处 | 低 | 编译失败 | 全局搜索 `state.http` 确认引用范围 |
| 环境变量读取移至配置后，已有部署需更新 `config.toml` | 中 | 启动失败 | `organization` / `project` 设为 `Option`，不配置则跳过 |

---

## 6. 与架构方案的对齐

本重构方案与 `docs/architecture/rust-code-architecture.md` 中的以下建议保持一致：

| 架构建议 | 本方案对应 |
|---------|-----------|
| `proxy` / `upstream` 负责上游请求转发与兼容性 | `UpstreamClient` 封装转发逻辑 |
| Handler 负责 HTTP 转换、参数校验、服务调用、统一响应 | `chat_completions` 拆分为编排 + 子函数 |
| 业务层与基础设施分离 | 解析逻辑从 handler 移至 `upstream.rs` |
| `ProxyService`：上游请求转发、兼容适配、流式转发 | `UpstreamClient` + `ProxyEndpoint` 为此铺路 |

---

**文档结束**
