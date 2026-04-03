# 审计日志开发实现方案

**版本:** 1.1
**编写日期:** 2026年4月1日（修订：2026年4月2日，对齐 [日志中心 UI/交互规格](../design/interaction/log-center.md)）
**适用范围:** ModelGate 审计日志功能的开发实现细节

> **维护提示：** 本文撰写时与代码对齐；若接口或字段变更，请同步更新 [开发 API](api.md)、[产品审计日志](../product/audit-log.md) 与 [实现状态](../implementation-status.md)。

---

## 1. 目的

本文件补充 `docs/architecture/audit-log-technical-solution.md` 的实现层面内容，给出具体代码模块、数据结构、数据库迁移、配置、请求拦截、异步写入、查询接口和权限控制方案。

---

## 2. 模块划分

- `src/config.rs`
  - 新增 `AuditConfig`
  - 支持审计文件目录、保留周期、批量写入参数、导出目录
- `src/main.rs`
  - 初始化 `tokio::sync::mpsc` 审计队列
  - 扩展 `AppState`，增加 `audit_sender` 与 `audit_config`
  - 注册审计中间件
  - 启动后台审计写入任务
- `src/audit.rs`
  - 定义 `AuditRecord` 和 `AuditMessage`
  - 实现请求/响应体写文件
  - 实现批量异步写入 SQLite
  - 实现失败重试与日志记录
- `src/db.rs`
  - 新增 `insert_audit_log`
  - 新增 `query_audit_logs`
  - 新增 `get_audit_log_by_request_id`
- `src/handlers/audit.rs`
  - 实现查询、详情、导出接口
- `src/routes.rs`
  - 注册审计日志相关路由

---

## 3. 配置设计

### 3.1 config.toml 示例

```toml
[audit]
log_dir = "./audit_logs"
retention_days = 90
batch_size = 50
flush_interval_seconds = 5
export_dir = "./exports"
```

### 3.2 `AuditConfig` 结构

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AuditConfig {
    pub log_dir: String,
    pub retention_days: u32,
    pub batch_size: usize,
    pub flush_interval_seconds: u64,
    pub export_dir: String,
}
```

---

## 4. 数据模型与迁移

### 4.1 SQLite 表结构

- 表名：`audit_logs`
- 主键：`request_id`

```sql
CREATE TABLE IF NOT EXISTS audit_logs (
    request_id TEXT PRIMARY KEY,
    user_id TEXT,
    token_id TEXT,
    channel_id TEXT,
    model TEXT,
    request_type TEXT,
    request_body_path TEXT,
    response_body_path TEXT,
    status_code INTEGER,
    error_message TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    cost REAL,
    latency_ms INTEGER,
    app_id TEXT,
    finish_reason TEXT,
    metadata TEXT,
    created_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs (created_at);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs (user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_token_id ON audit_logs (token_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_app_id ON audit_logs (app_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_finish_reason ON audit_logs (finish_reason);
```

**迁移说明：** 已有库需 `ALTER TABLE` 增加 `app_id`、`finish_reason`（允许 `NULL`）；老数据无值时列表展示 `—`。

### 4.2 扩展字段说明

- `metadata`：JSON 格式的扩展字段，包含：
  - `prompt_tokens_details`
  - `completion_tokens_details`
  - `cost_details`
  - `is_byok`
- **与日志中心 UI 对齐：** `app_id`、`finish_reason` 建议作为**表列**写入，便于列表筛选与索引；若过渡期仅存于 `metadata`，**列表/详情 API 仍应解析并返回扁平字段**（与前端表格、导出勾选一致）。

---

## 5. 核心结构设计

### 5.1 `AuditRecord`

```rust
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AuditRecord {
    pub request_id: String,
    pub user_id: Option<String>,
    pub token_id: Option<String>,
    pub channel_id: Option<String>,
    pub model: Option<String>,
    pub request_type: Option<String>,
    pub request_body_path: Option<String>,
    pub response_body_path: Option<String>,
    pub status_code: Option<u16>,
    pub error_message: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cost: Option<f64>,
    pub latency_ms: Option<i64>,
    pub app_id: Option<String>,
    pub finish_reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
}
```

- `app_id`：调用方应用标识（来源：配置映射、请求头如 `X-App-Id`、或 **API 密钥** 绑定属性，**以产品约定为准**）。
- `finish_reason`：从成功返回的 Chat/Completion 等 **JSON 响应体**解析（如 `choices[0].finish_reason`）；非此类请求、解析失败或错误响应则为 `None`。

### 5.2 `AuditMessage`

```rust
pub enum AuditMessage {
    Record(AuditRecord),
}
```

### 5.3 `AppState` 扩展

```rust
pub struct AppState {
    pub cfg: AppConfig,
    pub http: reqwest::Client,
    pub db: DbConn,
    pub audit_sender: tokio::sync::mpsc::Sender<AuditMessage>,
    pub audit_config: AuditConfig,
}
```

---

## 6. 审计中间件设计

### 6.1 中间件职责

- 生成 `request_id`
- 收集元信息：IP、User-Agent、请求路径、请求方法、请求体、用户身份、渠道、模型、类型、**应用标识（`app_id`）**
- 将请求体写入文件存储
- 记录请求开始时间
- 在请求完成后收集响应结果
- 发送 `AuditMessage` 到后台队列

### 6.2 中间件伪代码

```rust
async fn audit_middleware(
    req: ServiceRequest,
    srv: &dyn Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
) -> Result<ServiceResponse, Error> {
    let request_id = generate_request_id(); // 建议格式: {timestamp}_{random}
    let start = Instant::now();

    // 注意：读取请求体后必须重新放回 payload，避免影响后续 handler
    let (req, request_body) = read_and_restore_request_body(req).await?;
    let request_body_path = save_body_to_file(&state.audit_config, &request_id, "request", &request_body)?;

    let mut res = srv.call(req).await;
    let elapsed_ms = start.elapsed().as_millis() as i64;

    match &mut res {
        Ok(response) => {
            let status_code = response.status().as_u16();
            // 注意：读取响应体后必须重建 body 返回客户端
            let (response_body, rebuilt_response) = extract_and_rebuild_response_body(response).await?;
            *response = rebuilt_response;
            let response_body_path = save_body_to_file(&state.audit_config, &request_id, "response", &response_body)?;
            let audit_record = build_audit_record(...);
            let _ = state.audit_sender.send(AuditMessage::Record(audit_record)).await;
        }
        Err(err) => {
            let audit_record = build_audit_record_with_error(...);
            let _ = state.audit_sender.send(AuditMessage::Record(audit_record)).await;
        }
    }

    res
}
```

### 6.3 请求/响应体写文件

- 存储目录：`{log_dir}/{YYYY}/{MM}/{request_id}-{type}.json`
- 只保存路径到数据库
- 对于大请求体，建议限制单条最大大小并按需截断

---

## 7. 后台写入任务

### 7.1 任务职责

- 消费 `mpsc` 队列
- 批量写入数据库
- 按 `batch_size` 或超时 `flush_interval_seconds` 刷新
- 捕获写入错误并记录

### 7.2 任务伪代码

```rust
async fn audit_writer_loop(
    mut receiver: tokio::sync::mpsc::Receiver<AuditMessage>,
    db: DbConn,
    config: AuditConfig,
) {
    let mut buffer = Vec::new();
    let mut interval = tokio::time::interval(Duration::from_secs(config.flush_interval_seconds));

    loop {
        let mut should_flush = false;

        tokio::select! {
            msg = receiver.recv() => {
                match msg {
                    Some(AuditMessage::Record(record)) => {
                        buffer.push(record);
                        if buffer.len() >= config.batch_size {
                            should_flush = true;
                        }
                    }
                    None => {
                        // 通道关闭，退出前刷盘
                        should_flush = true;
                        if !buffer.is_empty() {
                            let _ = insert_audit_logs(&db, &buffer).await;
                            buffer.clear();
                        }
                        break;
                    }
                }
            }
            _ = interval.tick() => {
                // 超时也触发 flush，保证低流量下日志可及时落库
                should_flush = !buffer.is_empty();
            }
        }

        if should_flush && !buffer.is_empty() {
            if let Err(err) = insert_audit_logs(&db, &buffer).await {
                log::error!("audit insert failed: {err}");
            }
            buffer.clear();
        }
    }
}
```

### 7.3 失败保护

- 若数据库写入失败，可将审计消息写入本地日志文件或备用队列
- 报错应不影响主请求路径

---

## 8. 数据库接口

### 8.1 `insert_audit_log`

- 单条插入
- 批量插入

```rust
pub fn insert_audit_log(conn: &Connection, record: &AuditRecord) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO audit_logs (...) VALUES (...)",
        rusqlite::params![...],
    )?;
    Ok(())
}
```

### 8.2 `query_audit_logs`

- 支持分页参数：`limit`、`offset`
- 支持过滤条件：`created_at`、`user_id`、`token_id`、`channel_id`、`model`、`status_code`、`keyword`、**`app_id`**、**`finish_reason`（可多值 OR）**、**prompt/completion token 区间**（字段名与比较符以后端 query 契约为准，与 [日志中心 UI/交互规格](../design/interaction/log-center.md) 一致）
- `keyword` 用于匹配 `request_id`、`error_message`、`model` 等字段（建议使用 SQLite FTS5 或 `LIKE` + 索引组合）
- 单次查询最大 `limit=1000`，避免大范围全量扫描
- 仅返回元数据（默认不包含 request/response 路径）

### 8.3 `get_audit_log_by_request_id`

- 查询单条完整审计记录
- 可根据权限决定是否返回 `request_body_path` / `response_body_path`

---

## 9. 路由与 Handler

### 9.1 路由定义

```rust
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/logs")
            .route("/request", web::get().to(handlers::audit::list_audit_logs))
            .route("/request/{request_id}", web::get().to(handlers::audit::get_audit_log))
            .route("/export", web::post().to(handlers::audit::export_audit_logs))
            .route("/export/{export_id}", web::get().to(handlers::audit::get_export_status))
            .route("/export/{export_id}/download", web::get().to(handlers::audit::download_export_file)),
    );
}
```

### 9.2 `list_audit_logs`

- 解析查询参数（含时间范围、分页、排序及 **应用、`finish_reason`、token 区间** 等与 OpenAPI 一致的筛选项）
- 校验权限
- 调用 `query_audit_logs`
- 返回分页结果；**列表 DTO** 含 `prompt_tokens`、`completion_tokens`、`total_tokens`、`app_id`、`finish_reason` 等元数据字段（路径类字段仍按原策略不返回或脱敏）

### 9.3 `get_audit_log`

- 解析 `request_id`
- 校验权限
- 返回完整审计记录

### 9.4 `export_audit_logs`

- 支持 `format=json` 或 `format=csv`
- 使用后台任务生成导出文件
- 返回 `export_id` 和 `download_url`
- 新增 `get_export_status`：查询导出任务状态（processing/success/failed）
- 新增 `download_export_file`：导出完成后下载文件
- 权限策略：
  - 管理员可导出任意范围
  - 普通用户可导出本人日志（可通过配置关闭）

---

## 10. 文件存储与清理

### 10.1 文件存储规则

- `request_body` 保存为 `request-{request_id}.json`
- `response_body` 保存为 `response-{request_id}.json`
- 文件路径写入数据库
- 存储目录按年月分区

### 10.2 清理策略

- 定时任务扫描 `log_dir`
- 对保留期内日志默认只读，禁止更新/删除
- 保留期外按策略执行归档或删除（需符合合规策略）
- 删除超过 `retention_days` 的日志文件和对应数据库记录
- 建议增加文件 Hash 校验，支持篡改检测

---

## 11. 权限与安全

- 管理员可查看/导出任意审计记录
- 普通用户仅能查看自己的审计记录
- 普通用户可导出自己的审计记录（可配置关闭）
- 详情接口返回文件路径时应检查权限
- 请求内容和响应内容应按规则脱敏

---

## 12. 测试建议

- 单元测试：`insert_audit_log`、`query_audit_logs`、`get_audit_log_by_request_id`
- 集成测试：审计中间件是否生成 `request_id`、请求后是否发送队列、后台写入是否成功
- 安全测试：普通用户访问限制、导出权限
- 性能测试：高并发下 `mpsc` 队列和批量写入是否稳定
