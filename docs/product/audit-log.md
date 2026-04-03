# ModelGate 请求审计日志

**版本:** 1.2  
**更新日期:** 2026年4月

> **实现说明：** 列表/详情/导出等 API 已在服务端提供；字段与查询参数与本文基本一致。HTTP 路径与认证细节以 [开发 API](../development/api.md) 为准。存储路径、保留策略等运维项见 [部署文档](../development/deployment.md)。

---

## 一、产品概述

请求审计日志用于记录每一次 API 请求的完整信息，满足安全合规要求，支持问题排查和用量分析。

**核心目标：**
- 全链路请求记录，满足金融、政务合规要求
- 问题追踪与故障排查
- 用量分析与成本核算

---

## 二、记录内容

### 2.1 请求信息

| 字段 | 类型 | 说明 |
|------|------|------|
| request_id | string | 唯一请求ID，格式：`{timestamp}_{random}` |
| user_id | int | 用户ID |
| token_id | int | 令牌ID |
| channel_id | int | 渠道ID |
| model | string | 模型名称 |
| request_type | string | 请求类型：`chat`、`completion`、`embedding` 等 |
| app_id | string | 调用方应用标识（可选；可由请求头 `X-App-Id` 传入） |
| request_body | json | 请求体内容 |
| created_at | timestamp | 请求时间 |

### 2.2 响应信息

| 字段 | 类型 | 说明 |
|------|------|------|
| response_body | json | 响应体内容 |
| status_code | int | HTTP 状态码 |
| error_message | string | 错误信息（如有） |
| finish_reason | string | 模型完成原因（可选；如 Chat Completions 的 `choices[0].finish_reason`：`stop`、`length`、`content_filter`、`tool_calls` 等） |

### 2.3 用量与成本

| 字段 | 类型 | 说明 |
|------|------|------|
| prompt_tokens | int | 输入Token数 |
| completion_tokens | int | 输出Token数 |
| total_tokens | int | 总Token数 |
| cost | decimal | 消费金额（元） |
| latency_ms | int | 请求耗时（毫秒） |

### 2.4 用量详情（扩展）

| 字段 | 类型 | 说明 |
|------|------|------|
| prompt_tokens_details.cached_tokens | int | 缓存命中的输入Token（不收费） |
| prompt_tokens_details.cache_write_tokens | int | 写入缓存的Token数 |
| prompt_tokens_details.audio_tokens | int | 音频输入Token |
| prompt_tokens_details.video_tokens | int | 视频输入Token |
| completion_tokens_details.reasoning_tokens | int | 推理/思考Token |
| completion_tokens_details.image_tokens | int | 图像生成Token |
| completion_tokens_details.audio_tokens | int | 音频输出Token |
| is_byok | bool | 是否使用 BYOK 密钥 |

### 2.5 成本详情（扩展）

| 字段 | 类型 | 说明 |
|------|------|------|
| cost_details.upstream_inference_cost | decimal | 上游推理总成本 |
| cost_details.upstream_inference_prompt_cost | decimal | 上游输入推理成本 |
| cost_details.upstream_inference_completions_cost | decimal | 上游输出推理成本 |

**字段来源说明：**
- 基础字段（prompt_tokens, completion_tokens 等）用于配额扣减和统计
- 用量详情用于高级分析和缓存优化场景
- 成本详情用于成本分析和账单核算

---

## 三、功能特性

### 3.1 日志存储

**存储策略：**
- 按年分区存储，保障查询性能
- 默认保留 180 天
- 超期自动归档或删除（可配置）

**存储周期：**

| 场景 | 保留周期 | 说明 |
|------|---------|------|
| 通用场景 | 90 天 | 默认配置 |
| 金融/政务 | 180 天 | 满足合规要求 |
| 自定义 | 可配置 | 根据业务需求 |

### 3.2 日志查询

**支持维度：**
- 按时间范围筛选（Unix 时间戳，秒）
- 按用户/令牌/渠道筛选
- 按模型、`app_id`、**`finish_reason`**（逗号分隔多值，语义为 OR）筛选
- 按 **prompt / completion token 数量区间**筛选（`min_prompt_tokens`、`max_prompt_tokens` 等）
- 按状态码筛选（成功/错误）
- 关键词搜索

**查询限制：**
- 单次查询最大返回 1000 条
- 支持分页查询

### 3.3 日志导出

**导出格式：**
- JSON
- CSV

**导出方式：**
- 控制台直接下载
- API 接口触发导出

**导出字段（可配置）：**
- 基础信息：request_id、时间、用户、令牌、`app_id`、`finish_reason`
- 请求/响应：model、request_body、response_body（路径或正文依实现）
- 用量：prompt_tokens、completion_tokens、total_tokens、cost、latency

### 3.4 日志审计

**安全特性：**
- 日志不可删除（防篡改）
- 支持只读模式
- 操作日志独立记录

**合规特性：**
- 导出合规报告
- 满足等保/ISO 27001 要求

---

## 四、权限控制

### 4.1 角色权限

| 角色 | 查看日志 | 导出日志 | 删除日志 |
|------|:-------:|:-------:|:-------:|
| 超级管理员 | ✅ | ✅ | ✅ |
| 普通管理员 | ✅ | ✅ | ❌ |
| 普通用户 | 仅本人 | 仅本人 | ❌ |

### 4.2 访问控制

- 支持 IP 白名单访问控制
- 支持操作日志审计

---

## 五、API 接口

### 5.1 查询请求日志

**认证：** `Authorization: Bearer <API Key>`（控制台登录态与网关使用同一令牌时即为该 Key）

**常用 Query 参数：**

| 参数 | 说明 |
|------|------|
| `start_time` / `end_time` | Unix 时间戳（**秒**），按 `created_at` 过滤 |
| `limit` / `offset` | 分页；`limit` 默认 100，最大 1000 |
| `keyword` | 匹配 `request_id` / `error_message` / `model`（LIKE） |
| `model` | 精确匹配模型名 |
| `status_code` | 精确匹配 HTTP 状态码 |
| `user_id` / `token_id` / `channel_id` | 精确匹配（普通用户通常由服务端限定为本人） |
| `app_id` | 精确匹配应用标识 |
| `finish_reason` | 多个值用英文逗号分隔，语义为 **IN（OR）** |
| `min_prompt_tokens` / `max_prompt_tokens` 等 | Token 区间过滤 |

**请求示例：**
```
GET /api/v1/logs/request?start_time=1711843200&end_time=1711929600&limit=20&offset=0&app_id=my-app&finish_reason=stop,length
```

**响应：**
```json
{
  "data": [
    {
      "request_id": "1234567890_abc",
      "user_id": 1,
      "token_id": 10,
      "channel_id": null,
      "model": "gpt-4",
      "request_type": "chat",
      "status_code": 200,
      "error_message": null,
      "prompt_tokens": 100,
      "completion_tokens": 200,
      "total_tokens": 300,
      "cost": 0.015,
      "latency_ms": 1500,
      "app_id": "my-app",
      "finish_reason": "stop",
      "created_at": 1711920000
    }
  ],
  "total": 1000,
  "limit": 20,
  "offset": 0
}
```

### 5.2 导出请求日志

**请求：**
```
POST /api/v1/logs/export
Content-Type: application/json
Authorization: Bearer <API Key>

{
  "start_time": 1711843200,
  "end_time": 1711929600,
  "format": "csv"
}
```

> 当前实现为同步生成文件并返回 `success`；`fields` 自定义列为后续扩展。

**响应：**
```json
{
  "export_id": "exp_1234567890",
  "status": "success",
  "download_url": "/api/v1/logs/export/exp_1234567890/download"
}
```

---

## 六、计费与限制

### 6.1 存储限制

| 套餐 | 日志存储量 | 保留天数 |
|------|----------|---------|
| 免费版 | 100万条 | 30天 |
| 专业版 | 1000万条 | 90天 |
| 企业版 | 无限制 | 自定义 |

### 6.2 查询限制

| 套餐 | QPS 限制 | 并发限制 |
|------|---------|---------|
| 免费版 | 1 | 1 |
| 专业版 | 5 | 3 |
| 企业版 | 20 | 10 |

---

## 七、场景示例

### 7.1 问题排查

**场景：** 用户反馈 API 调用异常

**排查步骤：**
1. 根据用户反馈的时间范围查询日志
2. 按用户ID/令牌ID筛选
3. 查看错误信息和响应内容
4. 定位问题原因

### 7.2 合规审计

**场景：** 金融机构合规检查

**操作：**
1. 导出指定时间范围的完整日志
2. 生成合规报告
3. 提交审计

### 7.3 用量分析

**场景：** 部门成本核算

**操作：**
1. 按部门用户筛选日志
2. 统计各模型用量和成本
3. 导出报表

---

## 八、技术实现说明

技术方案已独立整理为开发技术文档，详见：

- [审计日志技术方案](../architecture/audit-log-technical-solution.md)

控制台「日志中心」界面与交互规格见：

- [日志中心 UI/交互规格](../design/interaction/log-center.md)

---

**文档结束**
