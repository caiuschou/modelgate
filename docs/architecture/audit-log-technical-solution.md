# 审计日志技术方案

**版本:** 1.1
**编写日期:** 2026年4月1日（修订：2026年4月2日）
**适用范围:** ModelGate 请求审计日志技术实现

> **落地情况：** 异步写入、落盘路径、列表/详情/导出 HTTP 接口等已按本方案主干实现；细节差异见 [开发 API](../development/api.md) 与源码 `src/audit.rs`、`src/handlers/audit.rs`。

---

## 1. 概述

本方案面向 ModelGate 审计日志能力，目标是实现：

- 全链路请求与响应审计
- 高性能异步写入，避免影响 API 响应
- 分区存储与生命周期管理
- 权限控制与合规审计
- 查询、导出、风险追踪

方案核心架构为“请求拦截 + 审计上下文采集 + 异步写入 + 分区存储”。

---

## 2. 架构设计

### 2.1 请求拦截与上下文采集

- 在 API 请求入口统一拦截请求。
- 生成唯一 `request_id`，用于全链路跟踪。
- 采集基础信息：`user_id`、`token_id`、`channel_id`、`model`、`request_type`、`request_body`。
- 记录调用元信息：来源 IP、渠道、时间戳、用户身份。

### 2.2 响应补充与统计字段

- 请求完成后补充响应信息：`response_body_file`/`response_body_path`、`status_code`、`error_message`。
- 请求体和响应体数据不直接存储在审计主表中，而是写入文件或对象存储，审计记录仅保存文件引用。
- 采集执行指标：`prompt_tokens`、`completion_tokens`、`total_tokens`、`cost`、`latency_ms`。
- 采集 **`finish_reason`**：对 JSON 类成功响应解析主 choice 的 `finish_reason`（与 OpenAI Chat Completions 语义对齐）；无法解析时留空。
- 采集 **`app_id`（应用标识）**：来自请求头、令牌元数据或路由配置（以产品约定为准）。
- 支持扩展字段：`prompt_tokens_details`、`completion_tokens_details`、`cost_details`、`is_byok`。

### 2.3 异步写入与性能保障

- 审计数据通过内部队列或后台任务异步写入数据库。
- 写入应为非阻塞流程，避免增加主请求路径延迟。
- 必要时可使用批量写入、缓冲区和失败重试机制。

---

## 3. 数据模型

### 3.1 审计日志主表字段

- `request_id`：唯一请求 ID，格式建议 `{timestamp}_{random}`。
- `user_id`：用户 ID。
- `token_id`：API 令牌 ID。
- `channel_id`：渠道 ID。
- `model`：模型名称。
- `request_type`：请求类型，如 `chat`、`completion`、`embedding`。
- `request_body_path`：请求体文件路径或文件引用。
- `response_body_path`：响应体文件路径或文件引用。
- `status_code`：HTTP 状态码。
- `error_message`：错误信息。
- `prompt_tokens`：输入 Token 数。
- `completion_tokens`：输出 Token 数。
- `total_tokens`：总 Token 数。
- `cost`：消费金额。
- `latency_ms`：请求耗时。
- `app_id`：调用方应用标识（可选，供控制台筛选与列表展示）。
- `finish_reason`：模型完成原因（可选，如 `stop`、`length`、`content_filter`、`tool_calls`）。
- `created_at`：请求时间。

### 3.2 扩展字段

- `prompt_tokens_details`：缓存命中、写入缓存、音频/视频 Token 等细节。
- `completion_tokens_details`：推理 Token、图像/音频输出 Token 等细节。
- `cost_details`：上游推理成本、输入成本、输出成本。
- `is_byok`：是否使用 BYOK 密钥。

---

## 4. 存储策略

### 4.1 分区与保留策略

- 建议按年月分区存储，例如 `audit_logs_2026_04`。
- 默认保留 90 天，金融/政务场景保留 180 天或更长。
- 支持自动归档与过期删除，归档数据可迁移到冷存储或对象存储。

### 4.2 数据一致性与只读保护

- 审计记录写入后尽量保持只读，防止普通用户删除或修改。
- 对请求体和响应体数据采用文件存储或对象存储，审计主表只保存文件路径/引用。
- 文件存储需与审计记录保持一致性，建议通过事务或二阶段提交保证文件引用与日志记录同步写入。
- 可增加 Hash 签名或校验机制，检测文件内容与审计记录是否篡改。
- 对敏感字段进行脱敏或掩码处理。

---

## 5. 查询与导出

### 5.1 查询接口

- 提供 `GET /api/v1/logs/request` 列表接口，用于获取审计记录元数据。
- 列表接口返回基础字段，不包含 `request_body_path` 和 `response_body_path`，以减少数据量并保护敏感内容。
- 支持按时间范围、用户、令牌、渠道、模型、状态码、关键词、**应用（`app_id`）**、**`finish_reason`**、**prompt/completion token 区间**筛选（具体 query 参数与 [日志中心 UI/交互规格](../design/interaction/log-center.md)、[审计日志开发实现](../development/audit-log-implementation.md) 保持一致）。
- 返回分页结果，单次最大返回条数限制。

### 5.2 详情接口

- 提供 `GET /api/v1/logs/request/{request_id}` 详情接口，用于获取单条审计记录完整信息。
- 详情接口返回包括 `request_body_path` 和 `response_body_path` 在内的完整审计字段。
- 该接口通常在需要追踪请求/响应内容、故障排查或合规审计时调用。
- 访问详情接口时应验证权限，避免未经授权访问敏感请求/响应数据。

### 5.3 导出接口

- 提供 `POST /api/v1/logs/export` 导出接口。
- 支持 JSON / CSV 格式。
- 支持导出字段自定义。
- 导出采用后台任务或异步导出，避免大数据量直接阻塞。
- 返回 `export_id` 与下载链接。

---

## 6. 权限与安全

### 6.1 角色权限设计

- 超级管理员：查看 / 导出 / 删除。
- 普通管理员：查看 / 导出。
- 普通用户：仅查看本人日志。

### 6.2 安全控制

- 支持 IP 白名单、只读审计访问。
- 管理操作单独记录操作审计日志。
- 若存在敏感信息，按规则脱敏存储。

---

## 7. 实施步骤

1. 设计阶段：确认字段模型、表结构、分区策略、保留周期。
2. 开发阶段：实现审计拦截、中间件、异步写入、查询导出接口。
3. 测试阶段：功能测试、性能测试、权限测试、边界场景测试。
4. 部署阶段：监控写入状态、存储容量、导出任务成功率。

---

## 8. 开发实现说明

本方案侧重架构与设计思路。具体的开发实现方案见：

- `docs/development/audit-log-implementation.md`

---

## 9. 关联说明

本技术方案独立于产品文档，面向开发与架构人员。产品文档中以概要说明为主，技术实现细节请参考本文件。
