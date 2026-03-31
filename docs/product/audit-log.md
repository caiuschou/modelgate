# ModelGate 请求审计日志

**版本:** 1.0  
**更新日期:** 2026年3月

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
| request_id | string | 唯一请求ID，格式：`req_{timestamp}_{random}` |
| user_id | int | 用户ID |
| token_id | int | 令牌ID |
| channel_id | int | 渠道ID |
| model | string | 模型名称 |
| request_type | string | 请求类型：`chat`、`completion`、`embedding` 等 |
| request_body | json | 请求体内容 |
| created_at | timestamp | 请求时间 |

### 2.2 响应信息

| 字段 | 类型 | 说明 |
|------|------|------|
| response_body | json | 响应体内容 |
| status_code | int | HTTP 状态码 |
| error_message | string | 错误信息（如有） |

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
- 按时间范围筛选
- 按用户/令牌/渠道筛选
- 按模型筛选
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
- 基础信息：request_id、时间、用户、令牌
- 请求/响应：model、request_body、response_body
- 用量：tokens、cost、latency

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

**请求：**
```
GET /api/v1/logs/request?start_time=2026-01-01&end_time=2026-03-31&user_id=1
```

**响应：**
```json
{
  "data": [
    {
      "request_id": "req_1234567890_abc",
      "user_id": 1,
      "token_id": 10,
      "model": "gpt-4",
      "status_code": 200,
      "prompt_tokens": 100,
      "completion_tokens": 200,
      "total_tokens": 300,
      "cost": 0.015,
      "latency_ms": 1500,
      "created_at": "2026-03-31T10:00:00Z"
    }
  ],
  "total": 1000,
  "page": 1,
  "page_size": 100
}
```

### 5.2 导出请求日志

**请求：**
```
POST /api/v1/logs/export
Content-Type: application/json

{
  "start_time": "2026-01-01",
  "end_time": "2026-03-31",
  "format": "csv",
  "fields": ["request_id", "user_id", "model", "cost"]
}
```

**响应：**
```json
{
  "export_id": "exp_1234567890",
  "status": "processing",
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

**文档结束**
