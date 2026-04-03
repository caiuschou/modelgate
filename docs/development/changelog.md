# ModelGate 更新日志

本文档记录面向使用方与贡献者的**显著变更**。细粒度提交历史见 Git。

格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循仓库 `Cargo.toml` / 前端 `package.json` 的语义化约定。

---

## [Unreleased]

### 文档

- 新增 `docs/implementation-status.md`：产品文档与代码实现对照。
- 新增 `docs/development/api.md`、`deployment.md`：当前服务端 API 与部署说明。
- 更新 `docs/index.md`、`README.md` 与多份产品/设计文档中的「实现范围」说明。

---

## [0.1.0] - 2026-04

### 后端

- Actix-web 网关：`/healthz`、注册/登录、`/v1/chat/completions` 上游代理（含流式）。
- SQLite 用户与 `api_keys`；登录返回 Bearer 用 API Key。
- 请求审计：列表、详情、导出及下载接口（`/api/v1/logs/*`）。
- 内测用 `POST /users`、`POST /users/{username}/keys`。

### 前端

- React 控制台：登录/注册、仪表盘、日志列表与详情；多模块占位路由。

### 工程

- Playwright E2E 与 CI 工作流（`.github/workflows/ci-e2e.yml`）。

---

**说明：** 早期若未打 Git tag，可将 `0.1.0` 视为「当前原型里程碑」的文档化标签，实际版本以发布流程为准。

---

**文档结束**
