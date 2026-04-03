# Rust 代码架构方案

**版本:** 1.0
**编写日期:** 2026年3月31日
**适用范围:** ModelGate Rust 原型项目

> **当前实现快照：** 仓库已具备 `proxy`、`session`、`user`、`audit` 等模块与 SQLite 存储；与下文「建议拆分」的理想分层可能部分重合、部分仍在演进。路由与接口以 [开发 API](../development/api.md) 为准。

---

## 0. 方案概览

本方案面向 ModelGate 产品核心目标：

- 统一接入多供应商大模型
- 提供 OpenAI 兼容 API
- 实现渠道管理、令牌管理、用量计费、负载均衡
- 支持高可用、成本可控、多租户、多场景部署

当前项目已经实现基础代理能力，下一步需要将业务需求转化为更明确的架构边界。

本方案建议按业务职责拆分：

- `auth` / `token` / `user` 负责访问控制与权限
- `channel` / `router` 负责模型路由与渠道调度
- `quota` / `billing` 负责配额与计费
- `proxy` / `upstream` 负责上游请求转发与兼容性
- `audit` / `logging` 负责全链路可观测与审计
- `metrics` / `telemetry` 负责监控与告警

这样的分层设计更贴合产品文档中“渠道管理、令牌管理、计费统计、负载均衡、日志审计”等核心能力。

---

## 1. 业务驱动模块划分

### 1.1 `src/main.rs`

保持最小职责：

- 加载配置
- 初始化日志与监控
- 初始化 DB 连接池、缓存、消息队列、HTTP 客户端
- 构建共享状态
- 注册路由与中间件
- 启动服务器

### 1.2 `src/config.rs`

配置应覆盖产品需求：

- 多渠道配置(`channels`)
- 管理接口开关
- 认证与权限规则
- 计费参数与倍率
- 白名单/域名限制
- 健康检查与熔断配置

建议支持：`config.toml`、`config.yaml`、环境变量、`dotenv`。

### 1.3 `src/db.rs`

建议生产级架构直接采用关系型数据库：

- `sqlx::PgPool` 或 `MySqlPool`
- 支持 `SqlitePool` 作为轻量原型
- 连接池+自动重试
- 统一迁移入口
- 事务支持

目标：为用户、令牌、渠道、用量、日志等核心数据提供稳定存储。

### 1.4 `src/models.rs`

领域模型应与业务功能直接对应：

- `Organization` / `Tenant`
- `User`
- `ApiToken` / `ApiKey`
- `UpstreamChannel`
- `ChannelHealth`
- `RouteRule`
- `QuotaRule`
- `UsageRecord`
- `RequestAudit`
- `BillingRecord`

这些模型将成为业务服务中的核心对象。

### 1.5 `src/repositories.rs`

持久化层承担以下职责：

- 用户/令牌/通道 CRUD
- 查询模型支持列表
- 计费与用量写入
- 日志审计写入
- 健康检查结果存储

推荐使用接口抽象，便于后续切换数据库或添加缓存层。

### 1.6 `src/services/mod.rs`

以业务能力为划分：

- `auth::AuthService`：令牌认证、身份、角色、IP/域名校验
- `token::TokenService`：令牌生成、吊销、过期校验、权限范围
- `channel::ChannelService`：渠道管理、优先级/权重、灰度、健康状态
- `router::RouteService`：模型匹配、智能调度、失败切换
- `quota::QuotaService`：配额计算、预警、限流、扣减逻辑
- `billing::BillingService`：Token/金额计费、用量统计、报表数据
- `audit::AuditService`：请求审计、日志记录、历史查询
- `proxy::ProxyService`：上游请求转发、兼容适配、流式转发

每个服务仅负责一类业务，按产品功能可直接映射到需求说明。

### 1.7 `src/handlers/*.rs`

建议将 Handler 分为：

- `handlers/public/`：对外模型调用接口，如 `/v1/chat/completions`、`/v1/embeddings`
- `handlers/admin/`：管理类接口，如渠道管理、令牌管理、用户管理、统计报表
- `handlers/health.rs`：健康检查与就绪探针

Handler 负责 HTTP 转换、参数校验、服务调用、统一响应。

### 1.8 `src/middleware.rs`

关键中间件：

- `AuthMiddleware`：解析 Bearer Token / api_key，注入认证信息
- `RateLimitMiddleware`：基于令牌/用户/IP 的速率限制
- `QuotaMiddleware`：检查令牌配额与消费阈值
- `CorsMiddleware`：跨域支持
- `ErrorMiddleware`：统一错误响应格式
- `RequestLoggingMiddleware`：记录请求元数据

这些能力正是产品文档中“认证方式、IP白名单、域名限制、限流配额”等功能点的基础。

### 1.9 `src/errors.rs`

建议统一错误体系，将错误映射为产品级 API 响应：

- `AuthError`：`missing_api_key`、`invalid_api_key`、`token_expired`、`token_revoked`、`ip_forbidden`
- `PermissionError`：`permission_denied`、`models_not_allowed`
- `QuotaError`：`quota_exceeded`、`quota_warning`
- `UpstreamError`：`upstream_timeout`、`upstream_failure`
- `ValidationError`：`invalid_parameters`
- `InternalError`

同时提供统一 JSON 结构，便于前端和 SDK 解析。

### 1.10 `src/telemetry.rs`

监控方案应覆盖业务指标和系统指标：

- 请求量、成功率、错误率
- 平均响应时延、尾部延迟
- 上游渠道调用次数、失败率
- 令牌使用量、配额消耗
- 每个模型 / 渠道的成本统计
- 渠道健康状态

建议支持 Prometheus 指标、Tracing、日志关联。

---

## 2. 建议目录结构

```
src/
  main.rs
  config.rs
  db.rs
  errors.rs
  telemetry.rs
  models.rs
  repositories.rs
  middleware.rs
  upstream.rs
  services/
    auth.rs
    token.rs
    channel.rs
    router.rs
    quota.rs
    billing.rs
    audit.rs
    proxy.rs
  handlers/
    health.rs
    public.rs
    admin.rs
  schema/
    migrations/
  support/
    metrics.rs
    health.rs
```

进一步扩展可按业务边界拆成：

- `src/domain/`：领域模型和业务规则
- `src/application/`：用例与服务编排
- `src/infrastructure/`：数据库、HTTP、缓存、消息队列
- `src/interfaces/`：HTTP API、管理 API

---

## 3. 请求处理流程

### 3.1 业务级请求流程

1. 客户端发起请求
2. 中间件解析 `Authorization: Bearer ...` 或 `api_key`
3. `AuthService` 验证令牌有效性和角色权限
4. `TokenService` 校验令牌是否被吊销、是否过期、是否命中 IP/域名限制
5. `QuotaService` 计算当前请求预算，检查配额是否允许继续执行
6. `ChannelService` 或 `RouteService` 根据模型、权重、优先级、渠道状态选择上游渠道
7. `ProxyService` 将请求转发给上游，并处理不同供应商兼容性
8. 记录 `UsageRecord`、`BillingRecord`、`AuditRecord`
9. 返回标准化的 OpenAI 兼容响应

### 3.2 业务场景映射

- 企业内部网关：`Organization`、`User`、`ApiToken`、`QuotaRule`
- API 分发：`BillingService`、`UsageRecord`、`AuditService`
- 成本控制：`ChannelService`、`RouteService`、`QuotaService`
- 多模型 A/B 测试：渠道权重、灰度规则、标签过滤
- 私有化部署：配置驱动、无外网依赖、可插拔上游适配器

### 3.3 流式请求处理

- `ProxyService` 识别 `stream=true`
- 使用 SSE / chunked 转发上游流
- 在流开始/结束时记录元数据
- 同步处理错误及关闭逻辑

---

## 4. 业务优化建议

### 4.1 渠道管理与调度

架构需支持产品文档的渠道能力：

- `UpstreamChannel` 包含 `name`、`provider`、`model_list`、`priority`、`weight`、`tags`
- 健康检查结果写入 `ChannelHealth`
- 支持渠道自动禁用、慢响应判定、故障摘除
- 路由规则按 `模型匹配 -> 优先级 -> 权重 -> 状态` 执行
- 支持灰度发布与用户组可见性

### 4.2 令牌与权限控制

令牌中心应支持：

- 虚拟令牌和原始 Key 隔离
- 过期时间、IP 白名单、域名白名单
- 模型可用范围与接口权限矩阵
- 令牌分组与批量管理
- 令牌异常调用检测与自动告警

### 4.3 计费与用量

计费模型应与产品描述一致：

- 输入/输出 Token 分开计量
- 不同模型倍率不同
- 支持按 Token、按金额、套餐与后付费
- 预算阈值触发预警
- 按组织/项目/令牌统计

### 4.4 审计与日志

日志与审计层应覆盖：

- 请求/响应完整链路
- 时间、Token、成本、渠道
- 错误原因与异常告警
- 查询与导出能力
- 与监控系统关联

### 4.5 多租户与用户管理

建议架构从一开始支持租户边界：

- `Organization` 或 `Tenant` 维度隔离
- 用户角色管理（超级管理员/普通管理员/普通用户）
- 组织内配额与成员权限
- 邀请、移除、审计历史

---

## 5. 核心改进建议

### 5.1 从 `rusqlite::Connection` 升级到连接池

当前实现使用 `Arc<Mutex<Connection>>` 会影响并发能力和扩展性。建议：

- 原型阶段使用 `SqlitePool`
- 生产阶段优先 `PostgreSQL` 或 `MySQL`
- 使用 `sqlx` 提供编译期 SQL 检查
- 对写密集场景引入 `Redis` 缓存/限流

### 5.2 业务层与基础设施分离

将业务服务与底层实现解耦：

- `services/` 只依赖接口和领域模型
- `repositories/` 封装数据库访问
- `upstream/` 封装 HTTP 转发与供应商兼容性
- `middleware/` 处理跨切关注点

这样有助于后续引入管理 UI、私有部署、第三方认证。

### 5.3 强化渠道与路由抽象

当前单一 `build_chat_completions_url` 不足以支持产品定位。建议：

- 定义 `ChannelProvider` trait
- 不同厂商按适配器实现 URL、Header、参数转换
- `RouteService` 支持权重、优先级、灰度、限流

### 5.4 统一错误与响应

使用统一错误层映射到产品级 API：

- `authentication_error`
- `rate_limit_error`
- `quota_exceeded`
- `permission_denied`
- `upstream_error`
- `internal_error`

为管理 API 和公开 API 统一返回格式，方便前端/SDK 集成。

### 5.5 强化监控与告警

建议尽早设计监控能力：

- 业务指标：令牌使用量、渠道成本、配额状态
- 系统指标：请求延迟、错误率、连接池饱和度
- 告警触发：渠道故障、配额预警、异常访问

---

## 6. 进化路线建议

### 6.1 短期目标

- 明确业务边界后拆分模块
- 补齐管理接口和统计接口
- 使用连接池与缓存
- 引入统一错误处理和监控

### 6.2 中期目标

- 支持多渠道智能路由与故障切换
- 完整实现令牌权限、IP/域名限制
- 实现 Token/金额计费和预算预警
- 实现审计日志与导出

### 6.3 长期目标

- 支持私有化部署与多租户隔离
- 支持 20+ 上游模型供应商适配
- 支持 A/B 测试与灰度渠道
- 支持管理控制台与运维告警

- 引入 Redis 缓存与限流
- 添加 Prometheus 指标

### 5.3 长期目标

- 可插拔上游适配器
- 支持管理后台配置动态刷新
- 支持多租户与分布式部署
- 支持更多 OpenAI API 兼容端点

---

## 6. 与现有代码的对比

当前现有实现已具备良好的原型基础：

- 配置管理集中在 `config.rs`
- `main.rs` 负责服务构建
- 已实现 `chat_completions` 和 `stream` 转发
- 使用 SQLite 进行 API Key 校验

但当前代码也存在以下集中点：

- `main.rs` 逻辑过重
- 数据库使用 `Arc<Mutex<Connection>>`
- 业务逻辑与路由耦合
- 错误处理分散
- 未封装上游渠道抽象

本方案正是为了解耦这些职责，并提供一个稳定、可扩展的 Rust 项目架构。
