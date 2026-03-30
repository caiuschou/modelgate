# ModelGate

> 统一大语言模型接入网关 - 一套 API，调用所有模型

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Go Version](https://img.shields.io/badge/Go-1.21+-00ADD8?logo=go)](https://golang.org/)
[![Documentation](https://img.shields.io/badge/docs-latest-green.svg)](docs/index.md)

---

## 📖 简介

ModelGate 是一款面向企业与开发者的大语言模型（LLM）统一接入网关系统。通过标准化的 OpenAI 兼容接口，ModelGate 将多个主流 LLM 供应商的能力聚合为统一的 API 服务，解决多模型管理混乱、成本不可控、调用复杂等核心痛点。

### ✨ 核心特性

- 🔌 **统一接入** — 支持 20+ 主流 LLM 供应商，一套 API 调用所有模型
- 🔑 **令牌管理** — 虚拟 API Key，精细化权限控制与配额管理
- ⚖️ **负载均衡** — 智能流量调度，故障自动切换，保障高可用
- 📊 **成本控制** — Token 级用量统计，预算预警，成本透明可控
- 🛡️ **安全合规** — 原始 Key 加密存储，多租户架构，审计日志

### 🎯 适用场景

- **企业内部网关** — 统一管理团队 API 使用，控制成本
- **API 二次分发** — 整合多家模型能力，对外提供付费 API 服务
- **成本控制与优化** — 精细化用量统计，预算管理，成本分摊
- **多模型 A/B 测试** — 快速对比不同模型效果，优化选择
- **私有化部署** — 数据不出内网，满足金融、医疗等行业合规要求

---

## 🚀 快速开始

### 本地开发（Rust 原型）

```bash
# 需要已安装 Rust（含 cargo）
cp config.example.toml config.toml

# 推荐用环境变量注入 Key（Windows PowerShell）
$env:UPSTREAM_API_KEY="sk-..."

# 启动
cargo run

# 测试（非流式）
curl http://localhost:8000/v1/chat/completions ^
  -H "Content-Type: application/json" ^
  -d "{\"model\":\"gpt-4.1\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello!\"}]}"
```

### 安装

```bash
# 使用 Docker
docker pull modelgate/server:latest
docker run -p 8080:8080 modelgate/server:latest

# 或下载二进制文件
wget https://github.com/yourusername/modelgate/releases/latest/download/modelgate-linux-amd64
chmod +x modelgate-linux-amd64
./modelgate-linux-amd64
```

### 配置

```bash
# 复制配置文件
cp config.example.yaml config.yaml

# 编辑配置
vim config.yaml
```

### 启动

```bash
# 启动服务
./modelgate server

# 访问管理界面
open http://localhost:8080
```

### 创建令牌

```bash
# 添加渠道
curl -X POST http://localhost:8080/api/channels \
  -H "Authorization: Bearer YOUR_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "OpenAI",
    "type": "openai",
    "key": "sk-xxx..."
  }'

# 创建令牌
curl -X POST http://localhost:8080/api/tokens \
  -H "Authorization: Bearer YOUR_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My App",
    "quota": 1000000
  }'
```

### 调用 API

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

---

## 📚 文档

- [产品概述](docs/product/overview.md) — 产品定位、核心功能、产品优势
- [功能详解](docs/product/features.md) — 渠道管理、令牌管理等详细功能
- [应用场景](docs/product/scenarios.md) — 企业内部网关、API 分发等场景
- [设计文档](docs/design/interaction/overview.md) — 交互设计规范、组件库
- [API 文档](docs/development/api.md) — API 接口说明

---

## 🛠️ 技术栈

- **后端**: Go 1.21+ / Gin / GORM
- **前端**: Vue 3 / TypeScript / Element Plus
- **数据库**: PostgreSQL / MySQL
- **缓存**: Redis
- **部署**: Docker / Kubernetes

---

## 🤝 贡献

欢迎贡献代码、报告问题或提出建议！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 提交 Pull Request

---

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

---

## 🙏 致谢

- [One API](https://github.com/songquanpeng/one-api) - 项目灵感来源
- [OpenAI](https://openai.com/) - OpenAI API 格式标准
- 所有贡献者 - 感谢您的贡献

---

## 📮 联系方式

- **官网**: https://modelgate.com
- **文档**: https://docs.modelgate.com
- **GitHub**: https://github.com/yourusername/modelgate
- **邮箱**: support@modelgate.com

---

<div align="center">

**⭐ 如果这个项目对你有帮助，请给我们一个 Star！**

</div>
