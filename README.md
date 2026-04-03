# ModelGate

> 统一大语言模型接入网关 — OpenAI 兼容 API、用户与密钥、请求审计（持续演进中）

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)](https://www.rust-lang.org/)
[![Documentation](https://img.shields.io/badge/docs-latest-green.svg)](docs/index.md)

---

## 简介

ModelGate 在本仓库中的形态是 **Rust（Actix-web）网关原型**：将客户端请求代理到**配置的单一上游** OpenAI 兼容服务，使用 SQLite 存储用户与 API Key，并记录可查询的**请求审计日志**。配套 **React + Vite** 管理控制台（登录/注册、日志中心等）。

**产品愿景与完整规格**（多渠道、负载均衡、计费、完整令牌 UI 等）见 [文档中心](docs/index.md)；**当前已实现能力**见 [实现状态](docs/implementation-status.md)。

---

## 快速开始

### 1. 后端

```bash
cp config.example.toml config.toml
# 设置上游密钥（推荐环境变量，勿提交真实 Key）
export UPSTREAM_API_KEY="sk-..."   # Windows PowerShell: $env:UPSTREAM_API_KEY="sk-..."
# 可选：覆盖邀请码
# export AUTH_INVITE_CODE="your-invite-code"

cargo run
```

默认监听 `http://127.0.0.1:8000`（见 `config.toml` 中 `[server]`）。

**健康检查：**

```bash
curl -s http://127.0.0.1:8000/healthz
```

**创建用户并获取 API Key（内测管理接口，勿对公网暴露）：**

```bash
curl -s -X POST http://127.0.0.1:8000/users \
  -H "Content-Type: application/json" \
  -d '{"username":"demo"}'
```

**调用 Chat Completions（需 Bearer 用户 Key）：**

```bash
curl http://127.0.0.1:8000/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4.1-mini","messages":[{"role":"user","content":"Hello!"}]}'
```

### 2. 前端控制台（可选）

```bash
cd frontend
npm ci
npm run dev
```

浏览器访问 `http://127.0.0.1:3000`。开发环境下 Vite 将 `/api`、`/healthz`、`/users` 代理到 `http://127.0.0.1:8000`。

---

## 文档

| 文档 | 说明 |
|------|------|
| [docs/index.md](docs/index.md) | 文档导航 |
| [docs/implementation-status.md](docs/implementation-status.md) | 文档与代码对照 |
| [docs/development/api.md](docs/development/api.md) | 已实现 HTTP API |
| [docs/development/deployment.md](docs/development/deployment.md) | 配置与部署 |
| [docs/product/overview.md](docs/product/overview.md) | 产品概述（目标能力） |

---

## 技术栈（本仓库）

- **后端：** Rust，Actix-web，reqwest，rusqlite（SQLite），bcrypt  
- **前端：** React 19，Vite，TypeScript，TailwindCSS，shadcn/ui，TanStack Query，Zustand  
- **E2E：** Playwright（CI：`.github/workflows/ci-e2e.yml`）

---

## 贡献

1. Fork 本仓库  
2. 创建特性分支（`git checkout -b feature/your-feature`）  
3. 提交更改并发起 Pull Request  

---

## 许可证

MIT — 见 [LICENSE](LICENSE)。

---

## 致谢

- [One API](https://github.com/songquanpeng/one-api) — 灵感来源  
- [OpenAI](https://openai.com/) — API 格式事实标准  
