# ModelGate API 文档

**版本:** 1.1  
**更新日期:** 2026年4月3日  
**文档类型:** API 产品规格说明书

> **重要：** 本文档描述 **OpenAI 兼容网关的目标产品规格**（多接口、用量 API、官方 SDK 示例等）。**本仓库当前已实现**的 HTTP 路由与认证行为以 [开发 API（当前实现）](../development/api.md) 与 `src/routes.rs` 为准。

---

## 一、API 概述

### 1.1 什么是 ModelGate API

ModelGate API 提供统一的 OpenAI 兼容接口，让您能够通过一套 API 调用多家大模型供应商的服务，无需关心底层的 API 差异。

### 1.2 核心优势

- **统一接口** — OpenAI 兼容格式，零学习成本
- **多模型支持** — 一套 API，调用 20+ 主流模型
- **智能路由** — 自动选择最优渠道，保障服务质量
- **成本透明** — 详细的用量统计和成本追踪
- **高可用性** — 负载均衡、故障自动切换

### 1.3 API 端点

| 环境 | 端点 |
|------|------|
| 生产环境 | `https://api.modelgate.com` |
| 测试环境 | `https://api-test.modelgate.com` |
| 私有部署 | `https://your-domain.com` |

---

## 二、快速开始

### 2.1 获取令牌

1. 注册 ModelGate 账号
2. 创建令牌（Token）
3. 复制令牌密钥（`sk-xxxxx...`）

### 2.2 发起第一个请求

```bash
curl https://api.modelgate.com/v1/chat/completions \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ]
  }'
```

### 2.3 响应示例

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "Hello! How can I help you today?"
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 9,
    "total_tokens": 19,
    "cost": 0.0015,
    "prompt_tokens_details": {
      "cached_tokens": 0,
      "cache_write_tokens": 0,
      "audio_tokens": 0,
      "video_tokens": 0
    },
    "cost_details": {
      "upstream_inference_cost": 0.0015,
      "upstream_inference_prompt_cost": 0.0005,
      "upstream_inference_completions_cost": 0.001
    },
    "completion_tokens_details": {
      "reasoning_tokens": 0,
      "image_tokens": 0,
      "audio_tokens": 0
    },
    "is_byok": false
  }
}
```

---

## 三、认证方式

### 3.1 Bearer Token 认证

所有 API 请求都需要在 HTTP Header 中包含令牌：

```http
Authorization: Bearer YOUR_TOKEN
```

**示例：**

```bash
curl https://api.modelgate.com/v1/models \
  -H "Authorization: Bearer sk-xxxxx"
```

### 3.2 API Key 认证（备选）

也可以通过 Query Parameter 传递令牌：

```bash
curl https://api.modelgate.com/v1/models?api_key=sk-xxxxx
```

> ⚠️ **注意：** 不推荐在生产环境使用 Query Parameter，因为令牌可能会被日志记录。

### 3.3 令牌权限

| 权限类型 | 普通令牌 | 管理员令牌 |
|---------|---------|-----------|
| 调用模型 API | ✅ | ✅ |
| 查看自己的用量 | ✅ | ✅ |
| 管理渠道 | ❌ | ✅ |
| 管理令牌 | ❌ | ✅ |
| 管理用户 | ❌ | ✅ |

---

## 四、接口列表

### 4.1 Chat Completions

创建聊天完成。

**端点：** `POST /v1/chat/completions`

**请求参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| model | string | 是 | 模型名称 |
| messages | array | 是 | 消息列表 |
| temperature | number | 否 | 温度参数 (0-2) |
| top_p | number | 否 | 核采样参数 |
| n | integer | 否 | 生成数量 |
| stream | boolean | 否 | 是否流式输出 |
| stop | string/array | 否 | 停止词 |
| max_tokens | integer | 否 | 最大生成 Token 数 |
| presence_penalty | number | 否 | 存在惩罚 (-2.0 到 2.0) |
| frequency_penalty | number | 否 | 频率惩罚 (-2.0 到 2.0) |

**请求示例：**

```json
{
  "model": "gpt-4",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "What is the capital of France?"}
  ],
  "temperature": 0.7,
  "max_tokens": 150
}
```

**响应示例：**

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "The capital of France is Paris."
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 9,
    "total_tokens": 29
  }
}
```

**流式响应示例：**

```bash
curl https://api.modelgate.com/v1/chat/completions \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": true
  }'
```

流式响应格式（SSE）：

```
data: {"id":"chatcmpl-123","choices":[{"delta":{"content":"Hello"},"index":0}]}

data: {"id":"chatcmpl-123","choices":[{"delta":{"content":"!"},"index":0}]}

data: [DONE]
```

---

### 4.2 Completions

创建文本完成（旧版接口，建议使用 Chat Completions）。

**端点：** `POST /v1/completions`

**请求参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| model | string | 是 | 模型名称 |
| prompt | string/array | 是 | 提示文本 |
| suffix | string | 否 | 后缀文本 |
| max_tokens | integer | 否 | 最大生成 Token 数 |
| temperature | number | 否 | 温度参数 |
| top_p | number | 否 | 核采样参数 |
| n | integer | 否 | 生成数量 |
| stream | boolean | 否 | 是否流式输出 |

**请求示例：**

```json
{
  "model": "gpt-3.5-turbo-instruct",
  "prompt": "Write a haiku about coding:",
  "max_tokens": 50,
  "temperature": 0.8
}
```

---

### 4.3 Embeddings

创建文本嵌入向量。

**端点：** `POST /v1/embeddings`

**请求参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| model | string | 是 | 嵌入模型名称 |
| input | string/array | 是 | 输入文本 |
| encoding_format | string | 否 | 编码格式 (float/base64) |
| dimensions | integer | 否 | 嵌入维度 |

**请求示例：**

```json
{
  "model": "text-embedding-ada-002",
  "input": "Hello, world!",
  "encoding_format": "float"
}
```

**响应示例：**

```json
{
  "object": "list",
  "data": [
    {
      "object": "embedding",
      "embedding": [0.0023, -0.0235, 0.0456, ...],
      "index": 0
    }
  ],
  "model": "text-embedding-ada-002",
  "usage": {
    "prompt_tokens": 4,
    "total_tokens": 4
  }
}
```

---

### 4.4 Models

列出可用模型。

**端点：** `GET /v1/models`

**响应示例：**

```json
{
  "object": "list",
  "data": [
    {
      "id": "gpt-4",
      "object": "model",
      "owned_by": "openai",
      "permission": []
    },
    {
      "id": "gpt-3.5-turbo",
      "object": "model",
      "owned_by": "openai",
      "permission": []
    },
    {
      "id": "claude-3-opus",
      "object": "model",
      "owned_by": "anthropic",
      "permission": []
    }
  ]
}
```

---

### 4.5 Images

生成图像。

**端点：** `POST /v1/images/generations`

**请求参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| prompt | string | 是 | 图像描述 |
| n | integer | 否 | 生成数量 (1-10) |
| size | string | 否 | 图像尺寸 |
| model | string | 否 | 模型名称 |
| quality | string | 否 | 图像质量 (standard/hd) |
| style | string | 否 | 图像风格 (vivid/natural) |

**请求示例：**

```json
{
  "model": "dall-e-3",
  "prompt": "A beautiful sunset over the ocean",
  "n": 1,
  "size": "1024x1024"
}
```

**响应示例：**

```json
{
  "created": 1677652288,
  "data": [
    {
      "url": "https://oaidalleapiprodscus.blob.core.windows.net/private/...",
      "revised_prompt": "A beautiful sunset over the ocean"
    }
  ]
}
```

---

### 4.6 Audio

**语音转文本：**

**端点：** `POST /v1/audio/transcriptions`

**请求参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| file | file | 是 | 音频文件 |
| model | string | 是 | 模型名称 |
| language | string | 否 | 语言代码 |
| prompt | string | 否 | 提示文本 |
| response_format | string | 否 | 响应格式 |
| temperature | number | 否 | 温度参数 |

**请求示例：**

```bash
curl https://api.modelgate.com/v1/audio/transcriptions \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -F file="@audio.mp3" \
  -F model="whisper-1"
```

**文本转语音：**

**端点：** `POST /v1/audio/speech`

**请求参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| model | string | 是 | 模型名称 |
| input | string | 是 | 输入文本 |
| voice | string | 是 | 语音 |
| response_format | string | 否 | 音频格式 |
| speed | number | 否 | 语速 (0.25-4.0) |

---

## 五、错误处理

### 5.1 错误响应格式

所有错误响应都遵循统一格式：

```json
{
  "error": {
    "message": "错误描述",
    "type": "错误类型",
    "param": null,
    "code": "错误码"
  }
}
```

### 5.2 HTTP 状态码

| 状态码 | 说明 |
|--------|------|
| 200 | 成功 |
| 400 | 请求参数错误 |
| 401 | 未授权（令牌无效或过期）|
| 402 | 配额不足 |
| 403 | 权限不足 |
| 404 | 资源不存在 |
| 429 | 请求过于频繁 |
| 500 | 服务器内部错误 |
| 502 | 上游服务错误 |
| 503 | 服务不可用 |
| 504 | 上游服务超时 |

### 5.3 错误类型

**认证错误：**

| 错误码 | 说明 | 解决方案 |
|--------|------|---------|
| invalid_api_key | API Key 无效 | 检查令牌是否正确 |
| token_expired | 令牌已过期 | 联系管理员续期 |
| token_revoked | 令牌已吊销 | 创建新令牌 |
| ip_not_allowed | IP 不在白名单 | 联系管理员添加 IP |

**配额错误：**

| 错误码 | 说明 | 解决方案 |
|--------|------|---------|
| quota_exceeded | 配额不足 | 充值或申请增加配额 |
| rate_limit_exceeded | 请求过于频繁 | 等待后重试 |

**请求错误：**

| 错误码 | 说明 | 解决方案 |
|--------|------|---------|
| invalid_model | 模型不存在 | 检查模型名称 |
| missing_parameter | 缺少必填参数 | 补充必填参数 |
| invalid_parameter | 参数格式错误 | 检查参数格式 |

### 5.4 错误处理最佳实践

```python
import requests

def chat_completion(messages):
    try:
        response = requests.post(
            "https://api.modelgate.com/v1/chat/completions",
            headers={
                "Authorization": f"Bearer {API_TOKEN}",
                "Content-Type": "application/json"
            },
            json={
                "model": "gpt-4",
                "messages": messages
            },
            timeout=30
        )
        response.raise_for_status()
        return response.json()

    except requests.exceptions.HTTPError as e:
        if e.response.status_code == 401:
            print("认证失败，请检查令牌")
        elif e.response.status_code == 402:
            print("配额不足，请充值")
        elif e.response.status_code == 429:
            print("请求过于频繁，请稍后重试")
        else:
            print(f"请求失败: {e.response.text}")
        return None

    except requests.exceptions.Timeout:
        print("请求超时，请稍后重试")
        return None

    except requests.exceptions.RequestException as e:
        print(f"网络错误: {str(e)}")
        return None
```

---

## 六、速率限制

### 6.1 限制规则

| 层级 | 默认限制 | 说明 |
|------|---------|------|
| 系统级 | 1000 QPS | 整体请求上限 |
| 用户级 | 100 QPS | 单用户请求上限 |
| 令牌级 | 可配置 | 单令牌请求上限 |
| 渠道级 | 可配置 | 单渠道请求上限 |

### 6.2 速率限制响应头

当触发速率限制时，响应会包含以下头信息：

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1677652288
Retry-After: 60
```

### 6.3 处理速率限制

```python
import time
import requests

def make_request_with_retry(url, data, max_retries=3):
    for attempt in range(max_retries):
        response = requests.post(url, json=data)

        if response.status_code == 429:
            retry_after = int(response.headers.get("Retry-After", 60))
            print(f"Rate limited. Waiting {retry_after} seconds...")
            time.sleep(retry_after)
            continue

        response.raise_for_status()
        return response.json()

    raise Exception("Max retries exceeded")
```

---

## 七、用量统计

### 7.1 查询用量

**端点：** `GET /v1/usage`

**请求参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| start_date | string | 是 | 开始日期 (YYYY-MM-DD) |
| end_date | string | 是 | 结束日期 (YYYY-MM-DD) |
| granularity | string | 否 | 粒度 (day/hour) |

**请求示例：**

```bash
curl "https://api.modelgate.com/v1/usage?start_date=2026-03-01&end_date=2026-03-30&granularity=day" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**响应示例：**

```json
{
  "object": "list",
  "data": [
    {
      "date": "2026-03-01",
      "requests": 1234,
      "prompt_tokens": 50000,
      "completion_tokens": 30000,
      "total_tokens": 80000,
      "cost": 12.50
    },
    {
      "date": "2026-03-02",
      "requests": 1456,
      "prompt_tokens": 60000,
      "completion_tokens": 35000,
      "total_tokens": 95000,
      "cost": 14.80
    }
  ]
}
```

### 7.2 实时用量

**端点：** `GET /v1/usage/realtime`

**响应示例：**

```json
{
  "today": {
    "requests": 5678,
    "prompt_tokens": 234567,
    "completion_tokens": 123456,
    "total_tokens": 358023,
    "cost": 56.80
  },
  "this_month": {
    "requests": 123456,
    "prompt_tokens": 5678901,
    "completion_tokens": 3456789,
    "total_tokens": 9135690,
    "cost": 1234.56
  },
  "quota": {
    "total": 10000000,
    "used": 9135690,
    "remaining": 864310,
    "usage_percentage": 91.36
  }
}
```

---

## 八、SDK 和集成

### 8.1 官方 SDK

**Python SDK:**

```bash
pip install modelgate-python
```

```python
import modelgate

client = modelgate.Client(api_key="YOUR_TOKEN")

response = client.chat.completions.create(
    model="gpt-4",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello!"}
    ]
)

print(response.choices[0].message.content)
```

**JavaScript SDK:**

```bash
npm install @modelgate/js
```

```javascript
import ModelGate from '@modelgate/js';

const client = new ModelGate({
  apiKey: 'YOUR_TOKEN'
});

const response = await client.chat.completions.create({
  model: 'gpt-4',
  messages: [
    { role: 'system', content: 'You are a helpful assistant.' },
    { role: 'user', content: 'Hello!' }
  ]
});

console.log(response.choices[0].message.content);
```

### 8.2 OpenAI SDK 兼容

ModelGate 完全兼容 OpenAI SDK，只需修改 base URL：

**Python:**

```python
from openai import OpenAI

client = OpenAI(
    api_key="YOUR_TOKEN",
    base_url="https://api.modelgate.com/v1"
)

response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello!"}]
)
```

**JavaScript:**

```javascript
import OpenAI from 'openai';

const openai = new OpenAI({
  apiKey: 'YOUR_TOKEN',
  baseURL: 'https://api.modelgate.com/v1'
});

const response = await openai.chat.completions.create({
  model: 'gpt-4',
  messages: [{ role: 'user', content: 'Hello!' }]
});
```

---

## 九、最佳实践

### 9.1 错误重试

```python
import time
from tenacity import retry, stop_after_attempt, wait_exponential

@retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=4, max=10)
)
def call_api_with_retry(messages):
    # 实现指数退避重试
    pass
```

### 9.2 流式处理

```python
def stream_completion(messages):
    response = requests.post(
        "https://api.modelgate.com/v1/chat/completions",
        json={
            "model": "gpt-4",
            "messages": messages,
            "stream": True
        },
        stream=True
    )

    for line in response.iter_lines():
        if line.startswith("data: "):
            data = line[6:]
            if data == "[DONE]":
                break
            chunk = json.loads(data)
            content = chunk["choices"][0]["delta"].get("content", "")
            print(content, end="", flush=True)
```

### 9.3 成本优化

1. **选择合适的模型**
   - 简单任务：使用 GPT-3.5 / DeepSeek
   - 复杂任务：使用 GPT-4 / Claude 3

2. **控制输出长度**
   ```python
   response = client.chat.completions.create(
       model="gpt-4",
       messages=messages,
       max_tokens=500  # 限制输出长度
   )
   ```

3. **使用缓存**
   - 对相同问题缓存结果
   - 减少重复调用

4. **监控用量**
   - 定期检查用量统计
   - 设置配额预警

### 9.4 安全建议

1. **保护令牌**
   - 不要在客户端代码中硬编码令牌
   - 使用环境变量或密钥管理服务
   - 定期轮换令牌

2. **使用 IP 白名单**
   - 限制令牌只能从特定 IP 访问
   - 提高安全性

3. **设置配额限制**
   - 防止意外超支
   - 控制使用量

4. **日志脱敏**
   - 不要在日志中记录完整令牌
   - 不要记录敏感数据

---

## 十、常见问题

### Q1: ModelGate 和直接调用 OpenAI 有什么区别？

**A:** ModelGate 提供以下优势：
- 统一接口调用多家模型
- 智能负载均衡和故障切换
- 详细的用量统计和成本控制
- 企业级权限管理

### Q2: 如何切换模型？

**A:** 只需修改 `model` 参数，无需修改代码：

```python
# 使用 GPT-4
response = client.chat.completions.create(
    model="gpt-4",
    messages=messages
)

# 切换到 Claude 3
response = client.chat.completions.create(
    model="claude-3-opus",
    messages=messages
)
```

### Q3: 流式响应如何使用？

**A:** 设置 `stream=True`：

```python
stream = client.chat.completions.create(
    model="gpt-4",
    messages=messages,
    stream=True
)

for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="")
```

### Q4: 如何查看我的用量？

**A:** 使用用量查询 API 或查看管理后台：

```python
usage = client.usage.query(
    start_date="2026-03-01",
    end_date="2026-03-30"
)
```

### Q5: 请求超时怎么办？

**A:**
1. 检查网络连接
2. 增加超时时间
3. 使用重试机制
4. 联系技术支持

---

## 十一、支持和反馈

- **文档**: https://docs.modelgate.com
- **API 状态页**: https://status.modelgate.com
- **技术支持**: support@modelgate.com
- **GitHub Issues**: https://github.com/yourusername/modelgate/issues
- **开发者社区**: https://community.modelgate.com

---

**文档结束**
