# ModelGate 文档中心

欢迎来到 ModelGate 文档中心。产品类文档描述**目标能力与市场规格**；与当前代码一致的内容见 **[实现状态](implementation-status.md)** 与 **[开发 API](development/api.md)**。

---

## 📚 文档导航

### 实现与交付（以代码为准）

- **[实现状态](implementation-status.md)** — 已实现 / 未实现对照表（后端路由、前端页面、CI）
- **[服务端 API（当前实现）](development/api.md)** — `src/routes.rs` 对应接口说明
- **[部署与运行](development/deployment.md)** — 配置、环境变量、前后端启动
- **[更新日志](development/changelog.md)** — 版本级变更摘要

### 产品文档

- **[产品概述](product/overview.md)** — 产品定位、核心功能、产品优势
- **[功能详解](product/features.md)** - 渠道管理、API 密钥管理、用户管理等详细功能说明
- **[应用场景](product/scenarios.md)** - 企业内部网关、API 分发、成本控制等典型场景
- **[API 文档（产品规格）](product/api.md)** - 目标态 API、认证、错误处理、SDK 示例（部分尚未在服务端实现）
- **[用户控制台（产品）](product/user-console.md)** - 管理门户能力规格
- **[API 密钥管理（产品）](product/api-key-management.md)** - 用户 Key 生命周期、权限与配额（规划）、与实现状态对照
- **[API 密钥能力增强（产品与交互规格）](product/api-key-capability-enhancement.md)** - 在现有 Key 能力上的分期增强、页面与流程级交互、API 草案与验收清单
- **[请求审计日志（产品）](product/audit-log.md)** - 字段说明、查询/导出 API、权限
- **[产品路线图](product/roadmap.md)** - MVP 定义、迭代计划、长期愿景

### 设计文档

- **[设计概述](design/interaction/overview.md)** - 设计目标、原则、用户角色
- **[注册页交互](design/interaction/register.md)** - 简单注册（用户名、密码、邀请码由服务端配置）
- **[导航设计](design/interaction/navigation.md)** - 全局导航、移动端导航、通知中心
- **[日志中心 UI/交互规格](design/interaction/log-center.md)** - 请求日志列表、详情、导出与权限
- **[设计令牌](design/interaction/design-tokens.md)** - 颜色、字体、间距、组件规范
- **[移动端设计](design/interaction/mobile.md)** - 移动端适配方案、触控交互

### 架构文档

- **[审计日志技术方案](architecture/audit-log-technical-solution.md)** - 审计日志设计与架构
- **[Rust 代码架构](architecture/rust-code-architecture.md)** - Rust 项目结构与组件设计
- **[前端架构](architecture/frontend-architecture.md)** - React + Vite + shadcn/ui + TailwindCSS 前端架构方案
- **[API 架构设计方案](architecture/api-architecture.md)** - 目标态扩展架构（文档内注明与当前 SQLite 原型差异）

### 开发文档

- **[服务端 API（当前实现）](development/api.md)** — 已实现 HTTP 接口
- **[部署与运行](development/deployment.md)** — 本地与简易生产
- **[更新日志](development/changelog.md)** — 版本变更摘要
- **[前端部署方案](development/frontend-deployment-plan.md)** - 前端 staging/prod 部署、回滚与运维规范
- **[前端开发计划](development/frontend-development-plan.md)** - 前端 8 周开发计划与里程碑
- **[前端开发约定](development/frontend-conventions.md)** - 命名、分层、提交流程与质量门禁
- **[前端 E2E 测试方案](development/e2e-testing-plan.md)** - Playwright、环境数据策略、用例范围；CI 见仓库 `.github/workflows/ci-e2e.yml`
- **[审计日志开发实现](development/audit-log-implementation.md)** - 审计日志开发实现方案
- **[代理处理器重构](development/proxy-handler-refactoring.md)** - proxy.rs 重构方案与实施计划
- **[开发计划（历史规划）](development-plan.md)** - 多阶段团队规划；与当前仓库栈不完全一致

### 其他

- **[文档整理说明](REORGANIZATION.md)** - 文档目录与迁移记录

---

## 🚀 快速开始

### 1. 了解产品

如果你是第一次了解 ModelGate，建议从以下文档开始：

1. [实现状态](implementation-status.md) - 区分「已上线」与「规划中」
2. [产品概述](product/overview.md) - 了解 ModelGate 是什么
3. [应用场景](product/scenarios.md) - 查看 ModelGate 能解决什么问题
4. [功能详解](product/features.md) - 深入了解核心功能（含目标验收标准）

### 2. 设计实施

如果你是设计师或产品经理，可以参考：

1. [设计概述](design/interaction/overview.md) - 了解设计原则和规范
2. [设计令牌](design/interaction/design-tokens.md) - 使用统一的设计规范
3. [导航设计](design/interaction/navigation.md) - 了解导航和交互模式

### 3. 开发集成

如果你是开发者，可以参考：

1. [服务端 API（当前实现）](development/api.md) - 对接本仓库网关
2. [部署与运行](development/deployment.md) - 启动与配置
3. [产品 API 规格](product/api.md) - 目标态接口与集成示例（实现进度见实现状态）

---

## 📋 文档版本

| 文档 | 版本 | 更新日期 |
|------|------|---------|
| 实现状态 | 1.0 | 2026-04-03 |
| 开发 API / 部署 / 更新日志 | 1.0 | 2026-04-03 |
| 产品概述 | 1.1 | 2026-04-03 |
| 功能详解 | 1.3 | 2026-04-03 |
| 请求审计日志（产品） | 1.1 | 2026-04-02 |
| 应用场景 | 1.1 | 2026-04-03 |
| 产品路线图 | 1.1 | 2026-04-03 |
| 用户控制台（产品） | 1.2 | 2026-04-03 |
| API 密钥管理（产品） | 1.0 | 2026-04-03 |
| 产品 API 规格 | 1.1 | 2026-04-03 |
| 设计概述 | 1.1 | 2026-04-03 |
| 导航设计 | 1.1 | 2026-04-03 |
| 日志中心 UI/交互规格 | 1.3 | 2026-04-02 |
| 设计令牌 | 1.0 | 2026-03-30 |
| 移动端设计 | 1.0 | 2026-03-30 |

---

## 💬 获取帮助

- **问题反馈**: [GitHub Issues](https://github.com/yourusername/modelgate/issues)
- **功能建议**: [GitHub Discussions](https://github.com/yourusername/modelgate/discussions)
- **邮件联系**: support@modelgate.com

---

## 📝 许可证

ModelGate 文档采用 [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) 许可证。

---

**最后更新:** 2026年4月3日
