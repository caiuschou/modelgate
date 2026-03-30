# ModelGate API 架构设计方案

**版本:** 1.0
**编写日期:** 2026年3月30日
**文档类型:** 技术架构设计
**技术栈:** Rust + Tokio + SQLx

---

## 一、架构概述

### 1.0 技术栈选型

#### 后端技术栈: Rust + Tokio + SQLx

ModelGate 采用 Rust 作为核心后端语言,充分利用 Rust 的性能和安全特性构建高可靠的 AI 模型网关系统。

**核心框架与库:**

| 分类 | 技术选型 | 说明 |
|------|---------|------|
| **Web框架** | Actix-web 4.x | 高性能异步Web框架,性能领先 |
| **运行时** | Tokio 1.x | 成熟稳定的异步运行时 |
| **数据库ORM** | SeaORM 0.12 | 类型安全的异步ORM框架 |
| **数据库驱动** | SQLx 0.7 | 编译期SQL检查的驱动库 |
| **缓存** | redis-rs 0.24 | Redis异步客户端 |
| **连接池** | deadpool-redis 0.14 | Redis连接池 |
| **消息队列** | rust-kafka 0.10 / Lapin 2.x | Kafka/RabbitMQ客户端 |
| **序列化** | Serde 1.0 | 零成本序列化/反序列化 |
| **日志** | Tracing 0.1 / tracing-subscriber 0.3 | 结构化日志和追踪 |
| **监控** | prometheus 0.13 / metrics 0.22 | Prometheus指标收集 |
| **限流** | governor 0.6 | 基于令牌桶的限流库 |
| **HTTP客户端** | Reqwest 0.11 | 异步HTTP客户端 |
| **WebSocket** | tokio-tungstenite 0.21 | WebSocket支持 |
| **加密** | aes-gcm 0.10 / bcrypt 0.15 | 对称加密和密码哈希 |
| **时间处理** | chrono 0.4 | 时间日期库 |
| **UUID** | uuid 1.6 | UUID生成 |
| **配置** | config 0.14 / dotenvy 0.15 | 配置管理 |
| **错误处理** | thiserror 1.0 / anyhow 1.0 | 错误定义和处理 |
| **搜索引擎** | elasticsearch 8.5 | Elasticsearch客户端 |
| **测试** | tokio-test 0.4 / criterion 0.5 | 异步测试和基准测试 |

**选择Rust作为后端语言的理由:**

1. **性能卓越**: Actix-web 在多项基准测试中领先,可处理数万 QPS
2. **内存安全**: 编译期保证内存安全,无 GC 延迟
3. **并发安全**: 所有权系统确保线程安全,无需复杂的锁机制
4. **类型安全**: 强类型系统在编译期捕获大部分错误
5. **零成本抽象**: 高级特性不影响运行时性能
6. **生态完善**: crates.io 提供丰富的库支持
7. **工具链优秀**: Cargo、rustfmt、clippy 等工具提升开发效率
8. **部署简单**: 单一二进制文件,无需复杂的运行时依赖

**选择Rust的优势:**

- **内存安全** - 编译期保证内存安全,避免空指针、数据竞争等问题
- **高性能** - 零成本抽象,接近C语言的性能
- **并发安全** - 所有权系统确保线程安全
- **低延迟** - 无GC, predictable performance
- **类型安全** - 强类型系统,编译期捕获大部分错误
- **生态完善** - Actix-web性能优异,Tokio运行时成熟稳定

---

### 1.1 设计目标

ModelGate API 系统旨在提供一个高性能、高可用、可扩展的多模型统一接入网关,核心设计目标包括:

- **统一接口** - OpenAI 兼容格式,零学习成本
- **高性能** - 支持高并发请求,低延迟响应
- **高可用** - 99.9%+ SLA,故障自动切换
- **可扩展** - 模块化设计,易于添加新模型和渠道
- **成本优化** - 智能路由,最小化调用成本
- **安全可靠** - 完善的认证授权和流量控制

### 1.2 架构原则

- **微服务架构** - 各组件独立部署,易于扩展
- **无状态设计** - API 服务无状态,支持水平扩展
- **异步处理** - 使用消息队列处理异步任务
- **缓存优先** - 多层缓存策略提升性能
- **防御编程** - 熔断、降级、限流保护系统稳定

---

## 二、系统架构

### 2.1 整体架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                         客户端层                                  │
│  Web 应用 / 移动应用 / 服务端应用 / 第三方集成                     │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                          接入层                                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐         │
│  │ CDN/WAF  │→ │ 负载均衡  │→ │ API 网关  │→ │ 限流网关  │         │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘         │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                         应用服务层                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ API 服务     │  │ 路由服务     │  │ 管理服务     │          │
│  │ - 认证授权   │  │ - 智能路由   │  │ - 渠道管理   │          │
│  │ - 请求处理   │  │ - 负载均衡   │  │ - 用户管理   │          │
│  │ - 响应转换   │  │ - 故障切换   │  │ - 配额管理   │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ 用量服务     │  │ 监控服务     │  │ 异步任务     │          │
│  │ - 统计分析   │  │ - 健康检查   │  │ - 日志处理   │          │
│  │ - 成本计算   │  │ - 性能监控   │  │ - 报表生成   │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                          数据层                                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐         │
│  │ Redis    │  │ MySQL    │  │ MongoDB  │  │ ClickHouse│        │
│  │ - 缓存   │  │ - 业务   │  │ - 日志   │  │ - 分析   │         │
│  │ - 限流   │  │ - 用户   │  │ - 事件   │  │ - 报表   │         │
│  │ - 会话   │  │ - 渠道   │  │          │  │          │         │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘         │
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                       │
│  │ Kafka    │  │ S3/MinIO │  │ Elastic  │                       │
│  │ - 消息   │  │ - 文件   │  │ - 日志   │                       │
│  │ - 事件   │  │ - 音视频 │  │ - 搜索   │                       │
│  └──────────┘  └──────────┘  └──────────┘                       │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                        上游模型层                                  │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐             │
│  │ OpenAI  │  │ Claude  │  │ Google  │  │ 国内厂商 │             │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘             │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 核心组件说明

#### 2.2.1 接入层

**CDN/WAF**
- 静态资源加速
- DDoS 防护
- WAF 规则过滤恶意请求

**负载均衡 (Nginx/ALB)**
- 四层/七层负载均衡
- 健康检查
- SSL 终结

**API 网关 (Kong/APISIX)**
- 统一入口
- 协议转换
- 请求/响应日志
- 插件扩展

**限流网关**
- 令牌桶算法
- 滑动窗口统计
- 多维度限流(IP/用户/令牌)

#### 2.2.2 应用服务层

**API 服务**
- 认证授权
- 请求验证
- 响应转换
- 流式处理

**路由服务**
- 模型映射
- 智能选路
- 负载均衡
- 故障切换

**管理服务**
- 渠道管理
- 用户管理
- 配额管理
- 令牌管理

**用量服务**
- 实时统计
- 成本计算
- 报表生成
- 预警通知

**监控服务**
- 健康检查
- 性能监控
- 告警规则
- 链路追踪

**异步任务**
- 日志处理
- 事件通知
- 定时任务
- 批量任务

---

## 三、核心流程设计

### 3.1 请求处理流程

```
客户端请求
    │
    ▼
┌─────────────┐
│ 1. CDN/WAF  │ - DDoS防护、WAF过滤
└─────────────┘
    │
    ▼
┌─────────────┐
│ 2. 负载均衡  │ - 分发到健康实例
└─────────────┘
    │
    ▼
┌─────────────┐
│ 3. API网关   │ - 路由、日志、插件
└─────────────┘
    │
    ▼
┌─────────────┐
│ 4. 限流检查  │ - 检查是否超限
└─────────────┘
    │
    ▼
┌─────────────┐
│ 5. 认证授权  │ - 验证令牌、权限
└─────────────┘
    │
    ▼
┌─────────────┐
│ 6. 请求验证  │ - 参数校验、模型映射
└─────────────┘
    │
    ▼
┌─────────────┐
│ 7. 缓存检查  │ - 查询是否有缓存
└─────────────┘
    │
    ├─ 有缓存 ──▶ 返回缓存结果
    │
    ▼ 无缓存
┌─────────────┐
│ 8. 智能路由  │ - 选择最优渠道
└─────────────┘
    │
    ▼
┌─────────────┐
│ 9. 上游调用  │ - 调用模型API
└─────────────┘
    │
    ▼
┌─────────────┐
│10. 响应处理  │ - 格式转换、错误处理
└─────────────┘
    │
    ▼
┌─────────────┐
│11. 异步记录  │ - 记录日志、更新用量
└─────────────┘
    │
    ▼
返回客户端
```

### 3.2 智能路由策略

#### 3.2.1 路由决策流程

```rust
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct Router {
    channels: Arc<RwLock<HashMap<i64, Channel>>>,
}

impl Router {
    pub async fn select_channel(&self, request: &Request) -> Result<Channel, Error> {
        // 1. 获取可用渠道列表
        let channels = self.get_available_channels(&request.model).await?;

        // 2. 过滤不符合条件的渠道
        let filtered = self.filter_channels(channels, request).await?;

        // 3. 按优先级排序
        let mut sorted = self.sort_channels(filtered, request).await?;

        // 4. 选择最优渠道
        sorted.into_iter()
            .next()
            .ok_or_else(|| Error::NoAvailableChannel)
    }

    async fn filter_channels(&self, channels: Vec<Channel>, request: &Request) -> Result<Vec<Channel>, Error> {
        let mut result = Vec::new();

        for channel in channels {
            // 检查渠道状态
            if !channel.is_healthy().await {
                continue;
            }

            // 检查配额
            if channel.quota_remaining.load(Ordering::Relaxed) <= 0 {
                continue;
            }

            // 检查限流
            if channel.is_rate_limited().await {
                continue;
            }

            // 检查用户权限
            if !channel.user_allowed(request.user_id).await {
                continue;
            }

            // 检查成本限制
            if channel.cost_per_token > request.max_cost {
                continue;
            }

            result.push(channel);
        }

        Ok(result)
    }

    async fn sort_channels(&self, channels: Vec<Channel>, request: &Request) -> Result<Vec<Channel>, Error> {
        let mut scored = Vec::new();

        for channel in channels {
            let score = self.calculate_channel_score(&channel, request).await;
            scored.push((channel, score));
        }

        // 按分数降序排序
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 返回排序后的渠道
        let sorted = scored.into_iter()
            .map(|(channel, _)| channel)
            .collect();

        Ok(sorted)
    }

    async fn calculate_channel_score(&self, channel: &Channel, _request: &Request) -> f64 {
        let mut score = 0.0;

        // 1. 成本优先 (权重: 40%)
        score += (1.0 - channel.cost_ratio) * 40.0;

        // 2. 性能优先 (权重: 30%)
        score += (1.0 - channel.avg_latency / 10000.0) * 30.0;

        // 3. 可用性优先 (权重: 20%)
        score += channel.success_rate * 20.0;

        // 4. 负载均衡 (权重: 10%)
        score += (1.0 - channel.load_ratio) * 10.0;

        score
    }
}
```

#### 3.2.2 负载均衡算法

**加权轮询 (Weighted Round Robin)**
```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct WeightedRoundRobin {
    channels: Vec<Channel>,
    weights: HashMap<i64, u32>,
    current_weights: Arc<Mutex<HashMap<i64, i64>>>,
}

impl WeightedRoundRobin {
    pub fn new(channels: Vec<Channel>) -> Self {
        let weights: HashMap<i64, u32> = channels.iter()
            .map(|c| (c.id, c.weight))
            .collect();

        let current_weights: HashMap<i64, i64> = channels.iter()
            .map(|c| (c.id, 0))
            .collect();

        Self {
            channels,
            weights,
            current_weights: Arc::new(Mutex::new(current_weights)),
        }
    }

    pub async fn next(&self) -> Option<Channel> {
        let total_weight: i64 = self.weights.values().map(|w| *w as i64).sum();

        // 更新当前权重
        let mut current = self.current_weights.lock().await;
        for channel in &self.channels {
            *current.get_mut(&channel.id).unwrap() += self.weights[&channel.id] as i64;
        }

        // 选择最大权重的渠道
        let selected = self.channels.iter()
            .max_by_key(|c| current.get(&c.id))
            .cloned();

        // 减少已使用的权重
        if let Some(ref channel) = selected {
            *current.get_mut(&channel.id).unwrap() -= total_weight;
        }

        selected
    }
}
```

**最少连接 (Least Connections)**
```rust
pub struct LeastConnections;

impl LeastConnections {
    pub fn select(channels: &[Channel]) -> Option<Channel> {
        channels.iter()
            .min_by_key(|c| c.active_requests.load(Ordering::Relaxed))
            .cloned()
    }
}
```

**一致性哈希 (Consistent Hash)**
```rust
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

pub struct ConsistentHash {
    replicas: usize,
    ring: BTreeMap<u64, Channel>,
    sorted_keys: Vec<u64>,
}

impl ConsistentHash {
    pub fn new(replicas: usize) -> Self {
        Self {
            replicas,
            ring: BTreeMap::new(),
            sorted_keys: Vec::new(),
        }
    }

    pub fn add_channel(&mut self, channel: Channel) {
        for i in 0..self.replicas {
            let key = Self::hash(&format!("{}:{}", channel.id, i));
            self.ring.insert(key, channel.clone());
            self.sorted_keys.push(key);
        }

        self.sorted_keys.sort();
        self.sorted_keys.dedup();
    }

    pub fn get_channel(&self, request_id: &str) -> Option<Channel> {
        if self.ring.is_empty() {
            return None;
        }

        let key = Self::hash(request_id);

        // 查找顺时针第一个节点
        if let Some((&ring_key, channel)) = self.ring.range(key..).next() {
            return Some(channel.clone());
        }

        // 环绕到第一个节点
        self.ring.values().next().cloned()
    }

    fn hash<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
}
```

### 3.3 故障切换机制

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, Instant};

pub struct FailoverManager {
    circuit_breakers: Arc<RwLock<HashMap<i64, CircuitBreaker>>>,
    retry_policies: HashMap<i64, RetryPolicy>,
}

impl FailoverManager {
    pub async fn call_with_fallback(&self, request: &Request) -> Result<Response, Error> {
        let channels = self.router.select_channels(request).await?;

        for (attempt, channel) in channels.into_iter().enumerate() {
            // 检查熔断器
            if self.is_circuit_open(&channel).await {
                continue;
            }

            match self.call_channel(&channel, request).await {
                Ok(response) => {
                    // 成功则更新熔断器
                    self.record_success(&channel).await;
                    return Ok(response);
                }
                Err(e) => {
                    // 记录失败
                    self.record_failure(&channel, &e).await;

                    // 最后一个渠道或不可重试
                    if attempt == channels.len() - 1 || !self.is_retryable(&e) {
                        return Err(e);
                    }

                    // 指数退避后重试下一个渠道
                    let delay = self.backoff_delay(attempt);
                    sleep(delay).await;
                }
            }
        }

        Err(Error::NoAvailableChannel)
    }

    async fn is_circuit_open(&self, channel: &Channel) -> bool {
        let breakers = self.circuit_breakers.read().await;
        let breaker = match breakers.get(&channel.id) {
            Some(b) => b,
            None => return false,
        };

        // 熔断器打开且未到恢复时间
        if breaker.state == BreakerState::Open {
            if breaker.opened_at.elapsed() < breaker.timeout {
                return true;
            } else {
                // 尝试半开
                drop(breakers);
                let mut breakers = self.circuit_breakers.write().await;
                if let Some(b) = breakers.get_mut(&channel.id) {
                    b.state = BreakerState::HalfOpen;
                }
                return false;
            }
        }

        false
    }

    async fn record_failure(&self, channel: &Channel, error: &Error) {
        let mut breakers = self.circuit_breakers.write().await;

        let breaker = breakers.entry(channel.id)
            .or_insert_with(|| CircuitBreaker::new(channel.id));

        breaker.failure_count += 1;
        breaker.last_error = Some(error.to_string());

        // 达到失败阈值,打开熔断器
        if breaker.failure_count >= breaker.threshold {
            breaker.state = BreakerState::Open;
            breaker.opened_at = Instant::now();
        }
    }

    async fn record_success(&self, channel: &Channel) {
        let mut breakers = self.circuit_breakers.write().await;

        if let Some(breaker) = breakers.get_mut(&channel.id) {
            breaker.failure_count = 0;

            if breaker.state == BreakerState::HalfOpen {
                breaker.state = BreakerState::Closed;
            }
        }
    }

    fn backoff_delay(&self, attempt: usize) -> Duration {
        // 指数退避: 2^attempt * 100ms
        Duration::from_millis((1 << attempt) * 100)
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub channel_id: i64,
    pub state: BreakerState,
    pub failure_count: u32,
    pub threshold: u32,
    pub timeout: Duration,
    pub opened_at: Instant,
    pub last_error: Option<String>,
}

impl CircuitBreaker {
    pub fn new(channel_id: i64) -> Self {
        Self {
            channel_id,
            state: BreakerState::Closed,
            failure_count: 0,
            threshold: 5,
            timeout: Duration::from_secs(60),
            opened_at: Instant::now(),
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}
```

---

## 四、数据模型设计

### 4.1 核心数据表

#### 4.1.1 用户表 (users)

```sql
CREATE TABLE users (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(100) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role ENUM('user', 'admin', 'super_admin') DEFAULT 'user',
    status ENUM('active', 'suspended', 'deleted') DEFAULT 'active',
    quota_total BIGINT DEFAULT 0,
    quota_used BIGINT DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    INDEX idx_email (email),
    INDEX idx_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

#### 4.1.2 令牌表 (tokens)

```sql
CREATE TABLE tokens (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    user_id BIGINT NOT NULL,
    token_key VARCHAR(255) UNIQUE NOT NULL,
    token_hash VARCHAR(255) NOT NULL,
    name VARCHAR(100),
    permissions JSON,
    ip_whitelist JSON,
    rate_limit INT DEFAULT 100,
    quota_limit BIGINT DEFAULT 0,
    expires_at TIMESTAMP NULL,
    last_used_at TIMESTAMP NULL,
    status ENUM('active', 'revoked', 'expired') DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_token_hash (token_hash),
    INDEX idx_user_id (user_id),
    INDEX idx_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

#### 4.1.3 渠道表 (channels)

```sql
CREATE TABLE channels (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(100) NOT NULL,
    provider VARCHAR(50) NOT NULL,
    type ENUM('openai', 'anthropic', 'google', 'azure', 'custom') NOT NULL,
    base_url VARCHAR(500) NOT NULL,
    api_key_encrypted TEXT NOT NULL,
    config JSON,
    models JSON,
    priority INT DEFAULT 0,
    weight INT DEFAULT 100,
    max_qps INT DEFAULT 10,
    status ENUM('active', 'disabled', 'error') DEFAULT 'active',
    health_status ENUM('healthy', 'degraded', 'unhealthy') DEFAULT 'healthy',
    last_check_at TIMESTAMP NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    INDEX idx_provider (provider),
    INDEX idx_status (status),
    INDEX idx_health_status (health_status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

#### 4.1.4 模型映射表 (model_mappings)

```sql
CREATE TABLE model_mappings (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    model_name VARCHAR(100) NOT NULL UNIQUE COMMENT '统一模型名称',
    aliases JSON COMMENT '模型别名',
    channel_id BIGINT NOT NULL COMMENT '默认渠道',
    model_id VARCHAR(100) NOT NULL COMMENT '渠道内模型ID',
    capabilities JSON COMMENT '模型能力',
    cost_per_1k_tokens DECIMAL(10,4) DEFAULT 0,
    max_tokens INT DEFAULT 4096,
    status ENUM('active', 'deprecated') DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    FOREIGN KEY (channel_id) REFERENCES channels(id),
    INDEX idx_model_name (model_name),
    INDEX idx_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

#### 4.1.5 请求日志表 (request_logs)

```sql
CREATE TABLE request_logs (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    request_id VARCHAR(100) UNIQUE NOT NULL,
    user_id BIGINT,
    token_id BIGINT,
    channel_id BIGINT,
    model VARCHAR(100),
    request_type VARCHAR(20),
    request_body JSON,
    response_body JSON,
    status_code INT,
    error_message TEXT,
    prompt_tokens INT DEFAULT 0,
    completion_tokens INT DEFAULT 0,
    total_tokens INT DEFAULT 0,
    cost DECIMAL(10,4) DEFAULT 0,
    latency_ms INT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_user_id (user_id),
    INDEX idx_token_id (token_id),
    INDEX idx_channel_id (channel_id),
    INDEX idx_model (model),
    INDEX idx_created_at (created_at),
    INDEX idx_status_code (status_code)
) PARTITION BY RANGE (YEAR(created_at)) (
    PARTITION p2026 VALUES LESS THAN (2027),
    PARTITION p2027 VALUES LESS THAN (2028),
    PARTITION pmax VALUES LESS THAN MAXVALUE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

#### 4.1.6 用量统计表 (usage_stats)

```sql
CREATE TABLE usage_stats (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    user_id BIGINT NOT NULL,
    token_id BIGINT,
    channel_id BIGINT,
    model VARCHAR(100),
    date DATE NOT NULL,
    hour TINYINT,

    request_count INT DEFAULT 0,
    success_count INT DEFAULT 0,
    error_count INT DEFAULT 0,

    prompt_tokens BIGINT DEFAULT 0,
    completion_tokens BIGINT DEFAULT 0,
    total_tokens BIGINT DEFAULT 0,

    cost DECIMAL(12,4) DEFAULT 0,

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE KEY uk_user_token_model_date_hour (user_id, token_id, model, date, hour),
    FOREIGN KEY (user_id) REFERENCES users(id),
    INDEX idx_user_id (user_id),
    INDEX idx_date (date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

### 4.2 Redis 数据结构

#### 4.2.1 限流计数器

```
# 令牌级限流
ratelimit:token:{token_id}:{window} = count

# 用户级限流
ratelimit:user:{user_id}:{window} = count

# IP 级限流
ratelimit:ip:{ip}:{window} = count

# 渠道级限流
ratelimit:channel:{channel_id}:{window} = count
```

#### 4.2.2 缓存数据

```
# 模型列表缓存
cache:models = {json_data}
TTL: 3600

# 渠道状态缓存
cache:channel:{channel_id}:status = {json_data}
TTL: 60

# 用户配额缓存
cache:user:{user_id}:quota = {json_data}
TTL: 300

# 响应缓存
cache:response:{hash} = {json_data}
TTL: 3600
```

#### 4.2.3 实时统计

```
# 实时用量
stats:usage:realtime:{user_id}:{date} = {
    "requests": 1234,
    "tokens": 567890,
    "cost": 123.45
}

# 渠道健康度
stats:channel:{channel_id}:health = {
    "success_rate": 99.5,
    "avg_latency": 1200,
    "error_count": 10
}

# 在线请求
stats:requests:active = {
    "total": 1234,
    "by_channel": {...}
}
```

---

## 五、安全架构

### 5.1 认证授权

#### 5.1.1 令牌认证

```rust
use actix_web::{dev::Payload, web, Error, FromRequest, HttpRequest};
use std::future::Future;
use std::pin::Pin;

pub struct TokenAuthenticator {
    db: Arc<DbPool>,
    redis: Arc<RedisPool>,
}

impl TokenAuthenticator {
    pub async fn authenticate(&self, request: &HttpRequest) -> Result<Token, AuthError> {
        // 1. 提取令牌
        let token_str = self.extract_token(request)?;
        if token_str.is_empty() {
            return Err(AuthError::MissingToken);
        }

        // 2. 查询令牌
        let token_obj = self.get_token(&token_str).await?;
        if token_obj.is_none() {
            return Err(AuthError::InvalidToken);
        }
        let token_obj = token_obj.unwrap();

        // 3. 检查令牌状态
        if token_obj.status != TokenStatus::Active {
            return Err(AuthError::TokenRevoked);
        }

        // 4. 检查过期时间
        if let Some(expires_at) = token_obj.expires_at {
            if expires_at < Utc::now() {
                return Err(AuthError::TokenExpired);
            }
        }

        // 5. 检查 IP 白名单
        let remote_ip = request.connection_info().realip_remote_addr().unwrap_or("");
        if !self.check_ip_whitelist(&token_obj, remote_ip).await? {
            return Err(AuthError::IPNotAllowed);
        }

        // 6. 更新最后使用时间
        self.update_last_used(token_obj.id).await?;

        Ok(token_obj)
    }

    fn extract_token(&self, request: &HttpRequest) -> Result<String, AuthError> {
        // 从 Header 提取
        if let Some(auth) = request.headers().get("Authorization") {
            let auth_str = auth.to_str().map_err(|_| AuthError::InvalidToken)?;
            if auth_str.starts_with("Bearer ") {
                return Ok(auth_str[7..].to_string());
            }
        }

        // 从 Query 参数提取
        if let Some(query) = request.uri().query() {
            let params: HashMap<String, String> = serde_urlencoded::from_str(query)
                .unwrap_or_default();
            if let Some(api_key) = params.get("api_key") {
                return Ok(api_key.clone());
            }
        }

        Err(AuthError::MissingToken)
    }

    async fn get_token(&self, token_str: &str) -> Result<Option<Token>, AuthError> {
        // 先从 Redis 缓存查询
        let cache_key = format!("token:{}", hash_token(token_str));
        if let Some(cached) = self.redis.get::<Token>(&cache_key).await? {
            return Ok(Some(cached));
        }

        // 从数据库查询
        let token = sqlx::query_as::<_, Token>(
            "SELECT * FROM tokens WHERE token_hash = ? AND status = 'active'"
        )
        .bind(hash_token(token_str))
        .fetch_optional(self.db.as_ref())
        .await?;

        // 缓存到 Redis
        if let Some(ref t) = token {
            self.redis.set(&cache_key, t, 300).await?;
        }

        Ok(token)
    }
}

// 实现 FromRequest trait,使 Token 可以直接作为控制器参数
impl FromRequest for Token {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let authenticator = req.app_data::<web::Data<TokenAuthenticator>>()
                .ok_or_else(|| Error::from(AuthError::MissingAuthenticator))?;

            authenticator.authenticate(&req).await
                .map_err(|e| Error::from(e))
        })
    }
}
```

#### 5.1.2 权限控制

```python
class PermissionChecker:
    def check_permission(self, token: Token, resource: str, action: str):
        # 获取权限列表
        permissions = token.permissions or []

        # 超级管理员拥有所有权限
        if token.user.role == 'super_admin':
            return True

        # 检查具体权限
        required = f"{resource}:{action}"
        if required in permissions:
            return True

        # 检查通配符权限
        wildcard = f"{resource}:*"
        if wildcard in permissions:
            return True

        raise PermissionDeniedError(
            f"Permission denied: {required}"
        )
```

### 5.2 数据加密

#### 5.2.1 API Key 加密存储

```python
from cryptography.fernet import Fernet

class KeyEncryption:
    def __init__(self, master_key: bytes):
        self.cipher = Fernet(master_key)

    def encrypt(self, api_key: str) -> str:
        """加密 API Key"""
        encrypted = self.cipher.encrypt(api_key.encode())
        return encrypted.decode()

    def decrypt(self, encrypted_key: str) -> str:
        """解密 API Key"""
        decrypted = self.cipher.decrypt(encrypted_key.encode())
        return decrypted.decode()
```

#### 5.2.2 敏感数据脱敏

```python
class DataMasking:
    @staticmethod
    def mask_token(token: str) -> str:
        """令牌脱敏"""
        if len(token) <= 8:
            return token[:4] + '****'
        return token[:7] + '...' + token[-4:]

    @staticmethod
    def mask_api_key(api_key: str) -> str:
        """API Key 脱敏"""
        if len(api_key) <= 8:
            return 'sk-****'
        return api_key[:7] + '****'

    @staticmethod
    def mask_email(email: str) -> str:
        """邮箱脱敏"""
        parts = email.split('@')
        if len(parts) != 2:
            return email

        name, domain = parts
        if len(name) <= 3:
            masked_name = name[:1] + '**'
        else:
            masked_name = name[:2] + '***' + name[-1:]

        return f"{masked_name}@{domain}"
```

### 5.3 流量安全

#### 5.3.1 限流策略

```rust
use redis::AsyncCommands;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct RateLimiter {
    redis: Arc<RedisPool>,
}

impl RateLimiter {
    pub fn new(redis: Arc<RedisPool>) -> Self {
        Self { redis }
    }

    pub async fn check_rate_limit(
        &self,
        user_id: i64,
        token_id: i64,
        limit: u64,
        window: u64,
    ) -> Result<bool, Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let key = format!("ratelimit:token:{}:{}", token_id, window);

        let mut conn = self.redis.get().await?;

        // 移除窗口外的记录
        let _: () = conn.zremrangebyscore(&key, 0, now as isize - window as isize).await?;

        // 添加当前请求
        let _: () = conn.zadd(&key, now as isize, now.to_string()).await?;

        // 设置过期时间
        let _: () = conn.expire(&key, window as usize).await?;

        // 获取当前计数
        let count: u64 = conn.zcard(&key).await?;

        Ok(count <= limit)
    }

    pub async fn get_rate_limit_info(
        &self,
        token_id: i64,
        window: u64,
    ) -> Result<RateLimitInfo, Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let key = format!("ratelimit:token:{}:{}", token_id, window);

        let mut conn = self.redis.get().await?;

        // 获取计数
        let count: u64 = conn.zcard(&key).await?;

        // 获取限制
        let limit = self.get_limit(token_id).await?;

        Ok(RateLimitInfo {
            limit,
            remaining: limit.saturating_sub(count),
            reset: now + window,
        })
    }

    async fn get_limit(&self, token_id: i64) -> Result<u64, Error> {
        // 从缓存或数据库获取令牌的限流配置
        let key = format!("token:limit:{}", token_id);
        let mut conn = self.redis.get().await?;

        let limit: Option<u64> = conn.get(&key).await?;

        if let Some(limit) = limit {
            Ok(limit)
        } else {
            // 从数据库查询
            let limit = sqlx::query_scalar::<_, u64>(
                "SELECT rate_limit FROM tokens WHERE id = ?"
            )
            .bind(token_id)
            .fetch_one(self.db.as_ref())
            .await?;

            // 缓存到 Redis
            let _: Option<()> = conn.set(&key, &limit).await?;

            Ok(limit)
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RateLimitInfo {
    pub limit: u64,
    pub remaining: u64,
    pub reset: u64,
}

// 使用 governor 库的内存限流器(作为备选方案)
use governor::{Quota, RateLimiter as GovernorLimiter};

pub struct MemoryRateLimiter {
    limiter: GovernorLimiter<&'static governor::state::direct::NotKeyed>,
}

impl MemoryRateLimiter {
    pub fn new(limit: u32, duration: Duration) -> Self {
        let quota = Quota::per_second(limit);
        let limiter = GovernorLimiter::direct(quota);

        Self { limiter }
    }

    pub fn check(&self) -> Result<(), Error> {
        self.limiter.check()?;
        Ok(())
    }
}
```

#### 5.3.2 DDoS 防护

```python
class DDoSProtection:
    def __init__(self, redis_client):
        self.redis = redis_client

    def check_request(self, ip: str) -> bool:
        """检查请求是否合法"""
        # 1. 检查 IP 信誉度
        if self.is_blacklisted(ip):
            return False

        # 2. 检查请求频率
        if self.exceeds_frequency_limit(ip):
            return False

        # 3. 检查异常行为
        if self.detect_anomaly(ip):
            return False

        return True

    def exceeds_frequency_limit(self, ip: str) -> bool:
        """检查是否超过频率限制"""
        key = f"ddos:ip:{ip}"

        pipe = self.redis.pipeline()
        pipe.incr(key)
        pipe.expire(key, 60)  # 60秒窗口

        count = pipe.execute()[0]

        # 1分钟内超过600次请求(10 QPS)
        return count > 600
```

---

## 六、性能优化

### 6.1 缓存策略

#### 6.1.1 多级缓存

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

pub struct MultiLevelCache {
    l1_cache: Arc<RwLock<HashMap<String, CacheEntry>>>,  // 进程内缓存
    l2_cache: Arc<RedisCache>,                          // Redis 缓存
    l3_cache: Arc<DatabaseCache>,                       // 数据库缓存
}

#[derive(Clone)]
struct CacheEntry {
    data: Vec<u8>,
    expires_at: Option<Instant>,
}

impl MultiLevelCache {
    pub fn new(l2_cache: Arc<RedisCache>, l3_cache: Arc<DatabaseCache>) -> Self {
        Self {
            l1_cache: Arc::new(RwLock::new(HashMap::new())),
            l2_cache,
            l3_cache,
        }
    }

    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, Error> {
        // L1: 进程内缓存
        {
            let l1 = self.l1_cache.read().await;
            if let Some(entry) = l1.get(key) {
                if let Some(expires_at) = entry.expires_at {
                    if expires_at > Instant::now() {
                        let data: T = bincode::deserialize(&entry.data)?;
                        return Ok(Some(data));
                    }
                } else {
                    let data: T = bincode::deserialize(&entry.data)?;
                    return Ok(Some(data));
                }
            }
        }

        // L2: Redis 缓存
        if let Some(data) = self.l2_cache.get::<T>(key).await? {
            // 回写到 L1
            self.set_l1(key, &data, None).await;
            return Ok(Some(data));
        }

        // L3: 数据库
        if let Some(data) = self.l3_cache.get::<T>(key).await? {
            // 回写到 L2
            self.l2_cache.set(key, &data, 3600).await?;

            // 回写到 L1
            self.set_l1(key, &data, None).await;

            return Ok(Some(data));
        }

        Ok(None)
    }

    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Option<u64>) -> Result<(), Error> {
        // 写入 L1
        let expires_at = ttl.map(|t| Instant::now() + Duration::from_secs(t));
        self.set_l1(key, value, expires_at).await;

        // 写入 L2
        self.l2_cache.set(key, value, ttl.unwrap_or(3600)).await?;

        Ok(())
    }

    async fn set_l1<T: Serialize>(&self, key: &str, value: &T, expires_at: Option<Instant>) -> Result<(), Error> {
        let data = bincode::serialize(value)?;
        let entry = CacheEntry { data, expires_at };

        let mut l1 = self.l1_cache.write().await;
        l1.insert(key.to_string(), entry);

        Ok(())
    }

    pub async fn invalidate(&self, key: &str) -> Result<(), Error> {
        // 从 L1 删除
        let mut l1 = self.l1_cache.write().await;
        l1.remove(key);

        // 从 L2 删除
        self.l2_cache.delete(key).await?;

        Ok(())
    }
}
```

#### 6.1.2 缓存预热

```rust
pub struct CacheWarmer {
    db: Arc<DbPool>,
    cache: Arc<MultiLevelCache>,
}

impl CacheWarmer {
    pub async fn warm_up(&self) -> Result<(), Error> {
        // 1. 预热模型列表
        self.warm_models().await?;

        // 2. 预热渠道配置
        self.warm_channels().await?;

        // 3. 预热用户配额
        self.warm_user_quotas().await?;

        Ok(())
    }

    async fn warm_models(&self) -> Result<(), Error> {
        let models = sqlx::query_as::<_, Model>(
            "SELECT * FROM model_mappings WHERE status = 'active'"
        )
        .fetch_all(self.db.as_ref())
        .await?;

        for model in models {
            let cache_key = format!("model:{}", model.id);
            self.cache.set(&cache_key, &model, Some(3600)).await?;
        }

        Ok(())
    }

    async fn warm_channels(&self) -> Result<(), Error> {
        let channels = sqlx::query_as::<_, Channel>(
            "SELECT * FROM channels WHERE status = 'active'"
        )
        .fetch_all(self.db.as_ref())
        .await?;

        for channel in channels {
            let cache_key = format!("channel:{}", channel.id);
            self.cache.set(&cache_key, &channel, Some(60)).await?;
        }

        Ok(())
    }

    async fn warm_user_quotas(&self) -> Result<(), Error> {
        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE status = 'active'"
        )
        .fetch_all(self.db.as_ref())
        .await?;

        for user in users {
            let cache_key = format!("user:{}:quota", user.id);
            let quota_info = QuotaInfo {
                total: user.quota_total,
                used: user.quota_used,
            };
            self.cache.set(&cache_key, &quota_info, Some(300)).await?;
        }

        Ok(())
    }
}
```

### 6.2 并发优化

#### 6.2.1 连接池管理

```rust
use reqwest::Client;
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct ConnectionPool {
    client: Arc<Client>,
    semaphore: Arc<Semaphore>,
}

impl ConnectionPool {
    pub fn new(max_connections: usize) -> Result<Self, Error> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(50)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()?;

        let semaphore = Arc::new(Semaphore::new(max_connections));

        Ok(Self {
            client: Arc::new(client),
            semaphore,
        })
    }

    pub async fn request(&self, method: Method, url: &str, body: Option<Value>) -> Result<Response, Error> {
        let _permit = self.semaphore.acquire().await?;

        let mut request = self.client.request(method, url);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        Ok(response)
    }

    pub async fn request_json(&self, method: Method, url: &str, body: Option<Value>) -> Result<Value, Error> {
        let response = self.request(method, url, body).await?;
        let json = response.json().await?;
        Ok(json)
    }
}
```

#### 6.2.2 异步任务处理

```rust
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub enum AsyncTask {
    Log(LogData),
    Stats(StatsData),
    Notification(NotificationData),
}

pub struct AsyncProcessor {
    sender: mpsc::Sender<AsyncTask>,
    workers: Vec<JoinHandle<()>>,
}

impl AsyncProcessor {
    pub fn new(num_workers: usize) -> Self {
        let (sender, mut receiver) = mpsc::channel(10000);

        let mut workers = Vec::new();

        for i in 0..num_workers {
            let mut rx = receiver.clone();

            let handle = tokio::spawn(async move {
                let name = format!("worker-{}", i);

                while let Some(task) = rx.recv().await {
                    if let Err(e) = Self::process_task(&name, task).await {
                        error!("{} error: {:?}", name, e);
                    }
                }
            });

            workers.push(handle);
        }

        Self { sender, workers }
    }

    pub async fn submit(&self, task: AsyncTask) -> Result<(), Error> {
        self.sender.send(task).await?;
        Ok(())
    }

    async fn process_task(worker_name: &str, task: AsyncTask) -> Result<(), Error> {
        match task {
            AsyncTask::Log(data) => {
                Self::save_log(data).await?;
            }
            AsyncTask::Stats(data) => {
                Self::update_stats(data).await?;
            }
            AsyncTask::Notification(data) => {
                Self::send_notification(data).await?;
            }
        }

        Ok(())
    }

    async fn save_log(data: LogData) -> Result<(), Error> {
        // 异步记录日志到数据库或日志服务
        Ok(())
    }

    async fn update_stats(data: StatsData) -> Result<(), Error> {
        // 异步更新统计数据
        Ok(())
    }

    async fn send_notification(data: NotificationData) -> Result<(), Error> {
        // 异步发送通知
        Ok(())
    }
}
```

### 6.3 数据库优化

#### 6.3.1 读写分离

```python
class DatabaseManager:
    def __init__(self, master_config, slave_configs):
        self.master = self.create_connection(master_config)
        self.slaves = [
            self.create_connection(config)
            for config in slave_configs
        ]
        self.slave_index = 0

    def get_master(self):
        """获取主库连接(写操作)"""
        return self.master

    def get_slave(self):
        """获取从库连接(读操作)"""
        slave = self.slaves[self.slave_index]
        self.slave_index = (self.slave_index + 1) % len(self.slaves)
        return slave
```

#### 6.3.2 批量写入

```python
class BatchWriter:
    def __init__(self, batch_size=1000, flush_interval=10):
        self.batch_size = batch_size
        self.flush_interval = flush_interval
        self.buffer = []
        self.last_flush = time.time()

    def add(self, item):
        """添加到缓冲区"""
        self.buffer.append(item)

        # 达到批量大小
        if len(self.buffer) >= self.batch_size:
            self.flush()

        # 超过时间间隔
        elif time.time() - self.last_flush > self.flush_interval:
            self.flush()

    def flush(self):
        """刷新到数据库"""
        if not self.buffer:
            return

        # 批量插入
        self.db.bulk_insert(self.buffer)

        # 清空缓冲区
        self.buffer = []
        self.last_flush = time.time()
```

---

## 七、监控和运维

### 7.1 监控指标

#### 7.1.1 系统指标

```rust
use sysinfo::{System, SystemExt, ProcessExt, CpuExt};
use prometheus::{IntGauge, Gauge};

pub struct SystemMonitor {
    sys: System,
    // Prometheus 指标
    cpu_usage: IntGauge,
    memory_usage: IntGauge,
    disk_usage: IntGauge,
    network_sent: IntGauge,
    network_recv: IntGauge,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let cpu_usage = IntGauge::new("system_cpu_usage_percent", "CPU usage percentage").unwrap();
        let memory_usage = IntGauge::new("system_memory_usage_percent", "Memory usage percentage").unwrap();
        let disk_usage = IntGauge::new("system_disk_usage_percent", "Disk usage percentage").unwrap();
        let network_sent = IntGauge::new("system_network_sent_mb", "Network sent MB").unwrap();
        let network_recv = IntGauge::new("system_network_recv_mb", "Network received MB").unwrap();

        Self {
            sys: System::new_all(),
            cpu_usage,
            memory_usage,
            disk_usage,
            network_sent,
            network_recv,
        }
    }

    pub fn collect_metrics(&mut self) {
        self.sys.refresh_all();

        // CPU 指标
        let cpu_usage = self.sys.global_cpu_info().cpu_usage() as i64;
        self.cpu_usage.set(cpu_usage);

        // 内存指标
        let memory_usage = (self.sys.used_memory() as f64 / self.sys.total_memory() as f64 * 100.0) as i64;
        self.memory_usage.set(memory_usage);

        // 磁盘指标
        for disk in self.sys.disks() {
            let usage = (disk.total_available_spaces() as f64 / disk.total_spaces() as f64 * 100.0) as i64;
            self.disk_usage.set(usage);
        }

        // 网络指标
        for (_name, data) in self.sys.networks() {
            let sent = data.total_transmitted() as i64 / (1024 * 1024);
            let recv = data.total_received() as i64 / (1024 * 1024);
            self.network_sent.set(sent);
            self.network_recv.set(recv);
        }
    }
}
```

#### 7.1.2 业务指标

```rust
use prometheus::{Counter, Histogram, IntGauge};
use lazy_static::lazy_static;

lazy_static! {
    // 请求指标
    static ref REQUESTS_TOTAL: Counter = Counter::new(
        "requests_total",
        "Total number of requests"
    ).unwrap();

    static ref REQUESTS_SUCCESS: Counter = Counter::new(
        "requests_success_total",
        "Total number of successful requests"
    ).unwrap();

    static ref REQUEST_DURATION: Histogram = Histogram::with_opts(
        Histogram::opts(
            "request_duration_seconds",
            "Request duration in seconds",
            vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
        ).unwrap()
    ).unwrap();

    // 用量指标
    static ref TOKENS_PROCESSED: Counter = Counter::new(
        "tokens_processed_total",
        "Total number of tokens processed"
    ).unwrap();

    static ref COST_TRACKER: Counter = Counter::new(
        "cost_total_dollars",
        "Total cost in dollars"
    ).unwrap();

    // 渠道指标
    static ref CHANNEL_HEALTH: IntGauge = IntGauge::new(
        "channel_healthy_count",
        "Number of healthy channels"
    ).unwrap();
}

pub struct BusinessMonitor;

impl BusinessMonitor {
    pub fn record_request(model: &str, latency: Duration, tokens: u64, cost: f64) {
        REQUESTS_TOTAL.inc();
        REQUESTS_SUCCESS.inc();
        REQUEST_DURATION.observe(latency.as_secs_f64());
        TOKENS_PROCESSED.inc_by(tokens);
        COST_TRACKER.inc_by(cost);

        // 添加标签
        let labels = [("model", model)];
        REQUESTS_TOTAL.with_label_values(&labels).inc();
    }

    pub fn update_channel_health(healthy_count: i64) {
        CHANNEL_HEALTH.set(healthy_count);
    }
}
```

### 7.2 告警规则

```yaml
alerts:
  - name: HighErrorRate
    condition: error_rate > 5%
    duration: 5m
    severity: critical
    message: "错误率过高: {{ error_rate }}%"

  - name: HighLatency
    condition: latency_p95 > 5000ms
    duration: 10m
    severity: warning
    message: "响应延迟过高: {{ latency_p95 }}ms"

  - name: ChannelUnhealthy
    condition: channel_success_rate < 90%
    duration: 3m
    severity: critical
    message: "渠道 {{ channel_name }} 不健康"

  - name: QuotaExceeded
    condition: quota_usage > 90%
    duration: 1m
    severity: warning
    message: "配额使用率超过 90%"
```

### 7.3 日志管理

#### 7.3.1 结构化日志

```rust
use tracing::{info, error, instrument};
use tracing_subscriber::{Registry, EnvFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_appender::rolling;

pub fn init_logging() {
    // 日志输出到文件
    let file_appender = rolling::daily("/var/log/modelgate", "api.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 配置 subscriber
    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into())
            .add_directive("sqlx=warn".parse().unwrap())
        )
        .with(tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(true)
        )
        .with(tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
        );

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
}

#[instrument(skip(request))]
pub async fn handle_request(request: Request) -> Result<Response, Error> {
    info!(
        request_id = %request.id,
        user_id = %request.user_id,
        model = %request.model,
        prompt_tokens = request.prompt_tokens,
        "request_received"
    );

    match process_request(&request).await {
        Ok(response) => {
            info!(
                request_id = %request.id,
                status_code = %response.status_code,
                latency_ms = response.latency_ms,
                total_tokens = response.total_tokens,
                "request_completed"
            );
            Ok(response)
        }
        Err(e) => {
            error!(
                request_id = %request.id,
                error_type = %std::any::type_name::<Error>(),
                error_message = %e,
                "request_failed"
            );
            Err(e)
        }
    }
}
```

#### 7.3.2 日志聚合

```rust
use elasticsearch::{Elasticsearch, IndexParts};
use serde_json::json;

pub struct LogAggregator {
    es: Elasticsearch,
}

impl LogAggregator {
    pub async fn aggregate_logs(&self, query: LogQuery) -> Result<LogAggregation, Error> {
        let response = self.es
            .search(SearchParts::Index(&["modelgate-logs-*"]))
            .body(json!({
                "query": query.filter,
                "aggs": {
                    "by_model": {
                        "terms": {
                            "field": "model.keyword",
                            "size": 20
                        }
                    },
                    "by_channel": {
                        "terms": {
                            "field": "channel_id",
                            "size": 20
                        }
                    },
                    "by_status": {
                        "terms": {
                            "field": "status_code",
                            "size": 10
                        }
                    },
                    "latency_stats": {
                        "stats": {
                            "field": "latency_ms"
                        }
                    }
                }
            }))
            .send()
            .await?;

        let response_body = response.json().await?;
        self.parse_aggregations(response_body)
    }

    fn parse_aggregations(&self, response: serde_json::Value) -> Result<LogAggregation, Error> {
        let aggregations = response["aggregations"].clone();

        Ok(LogAggregation {
            by_model: serde_json::from_value(aggregations["by_model"].clone())?,
            by_channel: serde_json::from_value(aggregations["by_channel"].clone())?,
            by_status: serde_json::from_value(aggregations["by_status"].clone())?,
            latency_stats: serde_json::from_value(aggregations["latency_stats"].clone())?,
        })
    }
}
```

---

## 八、部署架构

### 8.1 容器化部署

#### 8.1.1 Docker Compose

```yaml
version: '3.8'

services:
  # API 服务
  api:
    image: modelgate/api:latest
    ports:
      - "8000:8000"
    environment:
      - DATABASE_URL=mysql://user:pass@db:3306/modelgate
      - REDIS_URL=redis://redis:6379
    depends_on:
      - db
      - redis
    deploy:
      replicas: 3

  # 路由服务
  router:
    image: modelgate/router:latest
    environment:
      - DATABASE_URL=mysql://user:pass@db:3306/modelgate
      - REDIS_URL=redis://redis:6379
    depends_on:
      - db
      - redis
    deploy:
      replicas: 2

  # 数据库
  db:
    image: mysql:8.0
    environment:
      - MYSQL_ROOT_PASSWORD=rootpass
      - MYSQL_DATABASE=modelgate
    volumes:
      - db-data:/var/lib/mysql

  # Redis
  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data

  # Kafka
  kafka:
    image: confluentinc/cp-kafka:latest
    environment:
      - KAFKA_ZOOKEEPER_CONNECT=zookeeper:2181
    depends_on:
      - zookeeper

volumes:
  db-data:
  redis-data:
```

#### 8.1.2 Kubernetes 部署

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: modelgate-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: modelgate-api
  template:
    metadata:
      labels:
        app: modelgate-api
    spec:
      containers:
      - name: api
        image: modelgate/api:latest
        ports:
        - containerPort: 8000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: modelgate-secrets
              key: database-url
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: modelgate-secrets
              key: redis-url
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8000
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: modelgate-api-service
spec:
  selector:
    app: modelgate-api
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8000
  type: LoadBalancer
```

### 8.2 高可用部署

#### 8.2.1 多区域部署

```
                    ┌─────────────┐
                    │   DNS/GSLB  │
                    └─────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ▼                 ▼                 ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│  区域 A      │  │  区域 B      │  │  区域 C      │
│  (北京)      │  │  (上海)      │  │  (广州)      │
│              │  │              │  │              │
│  ┌────────┐  │  │  ┌────────┐  │  │  ┌────────┐  │
│  │ API    │  │  │  │ API    │  │  │  │ API    │  │
│  │ Router │  │  │  │ Router │  │  │  │ Router │  │
│  │ DB     │  │  │  │ DB     │  │  │  │ DB     │  │
│  │ Redis  │  │  │  │ Redis  │  │  │  │ Redis  │  │
│  └────────┘  │  │  └────────┘  │  │  └────────┘  │
└──────────────┘  └──────────────┘  └──────────────┘
        │                 │                 │
        └─────────────────┼─────────────────┘
                          │
                    数据同步(主从/复制)
```

---

## 九、扩展性设计

### 9.1 插件化架构

```python
class PluginManager:
    def __init__(self):
        self.plugins = {}

    def register(self, name: str, plugin: Plugin):
        """注册插件"""
        self.plugins[name] = plugin

    def execute_hook(self, hook_name: str, *args, **kwargs):
        """执行钩子"""
        results = []

        for plugin in self.plugins.values():
            if hasattr(plugin, hook_name):
                result = getattr(plugin, hook_name)(*args, **kwargs)
                results.append(result)

        return results

# 示例插件
class LoggingPlugin:
    def before_request(self, request):
        logger.info(f"Request: {request.id}")

    def after_request(self, request, response):
        logger.info(f"Response: {request.id} - {response.status_code}")

class MetricsPlugin:
    def after_request(self, request, response):
        metrics.record(
            model=request.model,
            latency=response.latency_ms,
            tokens=response.total_tokens
        )
```

### 9.2 模型适配器

```python
class ModelAdapter:
    """模型适配器基类"""
    def transform_request(self, request: Request) -> dict:
        """转换请求格式"""
        raise NotImplementedError

    def transform_response(self, response: dict) -> Response:
        """转换响应格式"""
        raise NotImplementedError

class OpenAIAdapter(ModelAdapter):
    def transform_request(self, request):
        return {
            "model": request.model_id,
            "messages": [
                {"role": m.role, "content": m.content}
                for m in request.messages
            ],
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        }

class AnthropicAdapter(ModelAdapter):
    def transform_request(self, request):
        return {
            "model": request.model_id,
            "messages": [
                {"role": m.role, "content": m.content}
                for m in request.messages
            ],
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        }

class AdapterFactory:
    adapters = {
        "openai": OpenAIAdapter,
        "anthropic": AnthropicAdapter,
        "google": GoogleAdapter,
    }

    @classmethod
    def get_adapter(cls, provider: str) -> ModelAdapter:
        adapter_class = cls.adapters.get(provider)
        if not adapter_class:
            raise UnsupportedProviderError(provider)

        return adapter_class()
```

---

## 十、Rust 项目结构

### 10.1 项目目录结构

```
modelgate/
├── Cargo.toml                          # 项目配置
├── Cargo.lock                          # 依赖锁定
├── .rustfmt.toml                       # 代码格式化配置
├── clippy.toml                         # Clippy lint 配置
│
├── src/                                # 源代码
│   ├── main.rs                         # 应用入口
│   ├── lib.rs                          # 库入口
│   │
│   ├── api/                            # API 层
│   │   ├── mod.rs
│   │   ├── handlers/                   # HTTP 处理器
│   │   │   ├── mod.rs
│   │   │   ├── chat.rs                 # 聊天接口
│   │   │   ├── completions.rs          # 补全接口
│   │   │   ├── embeddings.rs           # 嵌入接口
│   │   │   └── models.rs               # 模型列表
│   │   ├── middleware/                 # 中间件
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs                 # 认证中间件
│   │   │   ├── rate_limit.rs           # 限流中间件
│   │   │   └── logging.rs              # 日志中间件
│   │   └── routes.rs                   # 路由定义
│   │
│   ├── core/                           # 核心业务逻辑
│   │   ├── mod.rs
│   │   ├── router/                     # 路由服务
│   │   │   ├── mod.rs
│   │   │   ├── router.rs               # 路由器
│   │   │   ├── channel.rs              # 渠道管理
│   │   │   └── selector.rs             # 渠道选择器
│   │   ├── channel/                    # 渠道适配器
│   │   │   ├── mod.rs
│   │   │   ├── adapter.rs              # 适配器 trait
│   │   │   ├── openai.rs               # OpenAI 适配器
│   │   │   ├── anthropic.rs            # Anthropic 适配器
│   │   │   └── google.rs               # Google 适配器
│   │   └── failover/                   # 故障切换
│   │       ├── mod.rs
│   │       ├── circuit_breaker.rs      # 熔断器
│   │       └── retry.rs                # 重试策略
│   │
│   ├── models/                         # 数据模型
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── token.rs
│   │   ├── channel.rs
│   │   └── request_log.rs
│   │
│   ├── services/                       # 服务层
│   │   ├── mod.rs
│   │   ├── auth_service.rs
│   │   ├── quota_service.rs
│   │   ├── usage_service.rs
│   │   └── notification_service.rs
│   │
│   ├── repository/                     # 数据访问层
│   │   ├── mod.rs
│   │   ├── db.rs                       # 数据库连接
│   │   ├── user_repo.rs
│   │   ├── token_repo.rs
│   │   └── channel_repo.rs
│   │
│   ├── cache/                          # 缓存层
│   │   ├── mod.rs
│   │   ├── redis.rs                    # Redis 客户端
│   │   └── memory.rs                   # 内存缓存
│   │
│   ├── config/                         # 配置
│   │   ├── mod.rs
│   │   └── settings.rs                 # 应用配置
│   │
│   ├── error/                          # 错误处理
│   │   ├── mod.rs
│   │   └── types.rs                    # 错误类型定义
│   │
│   ├── telemetry/                      # 可观测性
│   │   ├── mod.rs
│   │   ├── metrics.rs                  # Prometheus 指标
│   │   └── tracing.rs                  # 分布式追踪
│   │
│   └── utils/                          # 工具函数
│       ├── mod.rs
│       ├── crypto.rs                   # 加密工具
│       └── validation.rs               # 验证工具
│
├── tests/                              # 测试
│   ├── integration/                    # 集成测试
│   └── fixtures/                       # 测试数据
│
├── migrations/                         # 数据库迁移
│   └── *.sql
│
├── docker/                             # Docker 配置
│   ├── Dockerfile
│   └── docker-compose.yml
│
├── k8s/                                # Kubernetes 配置
│   ├── deployment.yaml
│   ├── service.yaml
│   └── ingress.yaml
│
└── scripts/                            # 脚本
    ├── build.sh
    ├── deploy.sh
    └── migrate.sh
```

### 10.2 Cargo.toml 配置

```toml
[package]
name = "modelgate"
version = "1.0.0"
edition = "2021"
authors = ["ModelGate Team"]
description = "High-performance AI model gateway"

[dependencies]
# Web 框架
actix-web = "4.4"
actix-cors = "0.7"
actix-files = "0.6"

# 异步运行时
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# 数据库
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "mysql", "chrono", "uuid", "json"] }
sea-orm = { version = "0.12", features = ["sqlx-mysql", "runtime-tokio-rustls"] }

# Redis
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
deadpool-redis = "0.14"

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 日志和监控
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"
prometheus = "0.13"
metrics = "0.22"

# HTTP 客户端
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }

# 加密
aes-gcm = "0.10"
bcrypt = "0.15"

# 时间处理
chrono = { version = "0.4", features = ["serde"] }

# UUID
uuid = { version = "1.6", features = ["v4", "serde"] }

# 配置
config = "0.14"
dotenvy = "0.15"

# 错误处理
thiserror = "1.0"
anyhow = "1.0"

# 限流
governor = "0.6"

# Kafka (可选)
kafka = "0.10"

# Elasticsearch (可选)
elasticsearch = "8.5"

[dev-dependencies]
tokio-test = "0.4"
criterion = "0.5"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"

[profile.dev]
opt-level = 0
```

### 10.3 构建和部署

#### 本地开发

```bash
# 安装依赖
cargo fetch

# 运行开发服务器
cargo run

# 运行测试
cargo test

# 代码格式化
cargo fmt

# 代码检查
cargo clippy

# 构建优化版本
cargo build --release
```

#### Docker 构建

```dockerfile
# Dockerfile
FROM rust:1.75-alpine AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN apk add --no-cache musl-dev && \
    cargo build --release

FROM alpine:latest

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/target/release/modelgate /usr/local/bin/modelgate

EXPOSE 8000

CMD ["modelgate"]
```

#### 性能优化配置

```toml
# .cargo/config.toml
[build]
rustflags = ["-C", "target-cpu=native"]

[target.x86_64-unknown-linux-musl]
rustflags = ["-C", "link-arg=-s"]

[profile.release]
codegen-units = 1
lto = "fat"
opt-level = 3
strip = true
panic = "abort"
incremental = false
```

---

## 十一、总结

本架构设计方案为 ModelGate API 系统提供了基于 Rust 的完整技术蓝图,核心特点:

### 技术优势

1. **极致性能**
   - Rust 零成本抽象,接近 C 语言性能
   - 无垃圾回收, predictable latency
   - Actix-web 高性能异步框架,可处理数万 QPS
   - Tokio 异步运行时,高效并发处理

2. **内存安全**
   - 编译期保证内存安全
   - 无数据竞争、空指针等问题
   - 所有权系统确保线程安全
   - 减少运行时错误和调试成本

3. **高可用性**
   - 熔断降级、故障切换、多区域部署
   - Rust 类型系统减少运行时错误
   - 完善的错误处理机制
   - 99.9%+ SLA 保障

4. **可扩展性**
   - 微服务架构、插件化设计、模型适配器
   - Trait 系统支持灵活的抽象
   - 零成本抽象,性能不打折扣
   - 易于添加新模型和功能

5. **安全可靠**
   - 完善认证、数据加密、流量控制
   - Rust 安全特性防止常见漏洞
   - 类型安全的序列化/反序列化
   - 编译期检查减少安全漏洞

6. **易于运维**
   - 监控告警、日志管理、自动化部署
   - 单一二进制文件,部署简单
   - 结构化日志和链路追踪
   - Prometheus 集成开箱即用

### 技术栈亮点

- **Actix-web**: 性能优异的 Web 框架,基准测试领先
- **Tokio**: 成熟稳定的异步运行时
- **SQLx**: 编译期检查 SQL,避免运行时错误
- **Serde**: 零成本序列化,性能最佳
- **Tracing**: 结构化日志和分布式追踪
- **Prometheus**: 云原生监控标准

### 开发体验

- **Cargo**: 强大的包管理和构建工具
- **编译器**: 友好的错误提示
- **工具链**: rustfmt、clippy、docs.rs
- **生态**: crates.io 丰富的库支持

通过本架构方案的实施,ModelGate 能够在保证极致性能的同时,提供稳定可靠的 AI 模型 API 网关服务,支持大规模生产环境部署。

---

**参考资源:**

- [Rust 官方文档](https://www.rust-lang.org/)
- [Actix-web 文档](https://actix.rs/)
- [Tokio 运行时](https://tokio.rs/)
- [SQLx 文档](https://docs.rs/sqlx/)
- [Tracing 日志](https://docs.rs/tracing/)
- [Prometheus 客户端](https://docs.rs/prometheus/)
