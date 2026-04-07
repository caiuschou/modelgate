# 团队与成员功能开发方案

**版本:** 1.0  
**编写日期:** 2026年4月7日  
**适用范围:** ModelGate 控制台与网关后端的「组织协作」能力（后续里程碑）

> **产品前提：** 本功能**不属于**当前 v1 契约范围，见 [产品范围与原则](../product/product-scope-principles.md)。交互与页面规格见 [团队与成员交互规格](../design/interaction/teams-and-members.md)；功能级目标见 [功能详解 §3.2](../product/features.md#32-组织与成员团队--多用户协作)。

> **维护提示：** 路由落地后同步 [开发 API](api.md)、[实现状态](../implementation-status.md)；合并后更新 [前端开发计划](frontend-development-plan.md) 对应 Feature 行。

---

## 1. 目的

在保留现有 **单用户账户 + JWT/API Key 会话** 的前提下，引入 **团队（Team）** 与 **成员关系（Membership）**，使 API 密钥、审计日志等资源可按 **团队上下文** 隔离与协作管理。本方案给出 **数据库迁移、后端模块划分、团队上下文传递、与现有 `/api/v1/me/*` 及审计链路的衔接**，以及前端与测试门禁。

---

## 2. 分期建议

| 阶段 | 目标 | 交付物（验收） |
|------|------|----------------|
| **A. 数据与领域** | 表结构、仓储与纯逻辑单测 | 迁移脚本 + `services`/`db` 层 CRUD；无 HTTP |
| **B. 团队 API** | 团队 CRUD、成员列表、角色变更、邀请/接受 | `routes.rs` 注册；集成测试或 handler 测试 |
| **C. 资源归属** | Key 与审计带 `team_id`；列表按团队过滤 | 迁移扩列；代理与审计写入读团队上下文 |
| **D. 控制台** | 切换器、成员页、邀请流 | React 路由与 Playwright E2E |

C 可与 B 并行设计，但**上线建议**先 B 再 C，避免半截数据模型（先团队后挂资源更干净）。

---

## 3. 数据模型与迁移

### 3.1 新表（SQLite）

**`teams`**

| 列 | 类型 | 说明 |
|----|------|------|
| `id` | INTEGER PK | 自增 |
| `name` | TEXT NOT NULL | 展示名 |
| `slug` | TEXT NOT NULL UNIQUE | URL 友好，小写连字符，应用层校验格式 |
| `created_by_user_id` | INTEGER NOT NULL FK → users | 创建者 |
| `created_at` | INTEGER NOT NULL | Unix 秒 |

索引：`idx_teams_created_by`（可选，按创建者查）。

**`team_members`**

| 列 | 类型 | 说明 |
|----|------|------|
| `team_id` | INTEGER NOT NULL FK → teams ON DELETE CASCADE |
| `user_id` | INTEGER NOT NULL FK → users ON DELETE CASCADE |
| `role` | TEXT NOT NULL | `owner` \| `admin` \| `member`（应用层枚举，禁止随意字符串） |
| `joined_at` | INTEGER NOT NULL | Unix 秒 |

主键：`(team_id, user_id)` 联合唯一。

**`team_invitations`**

| 列 | 类型 | 说明 |
|----|------|------|
| `id` | INTEGER PK | 自增 |
| `team_id` | INTEGER NOT NULL FK → teams |
| `email` | TEXT | 可空：若仅用用户名邀请则另列 `username`；与产品一致后二选一或并存 |
| `role` | TEXT NOT NULL | 邀请授予角色（不含 owner） |
| `token_hash` | TEXT NOT NULL UNIQUE | 仅存哈希，不入库明文 token |
| `created_by_user_id` | INTEGER NOT NULL |
| `created_at` | INTEGER NOT NULL |
| `expires_at` | INTEGER NOT NULL |
| `accepted_at` | INTEGER | 可空 |

索引：`idx_team_invites_team`；有效邀请查询可按 `token_hash` + `expires_at`。

### 3.2 已有表扩列（阶段 C）

**`api_keys`**

- `team_id` INTEGER NULL FK → teams(id) ON DELETE SET NULL（或 CASCADE，按产品：**删团队是否删 Key** 需产品拍板；建议 **CASCADE** 与「团队为治理边界」一致）。
- 语义：`team_id IS NULL` = **个人密钥**（与 [交互文档§7](../design/interaction/teams-and-members.md#七与个人资源共存可选策略) 一致）；非空则归属团队。

**`audit_logs`**

- `team_id` INTEGER NULL FK → teams(id)（可选 ON DELETE SET NULL，避免删团队丢历史）。
- 写入路径：`src/audit.rs` / 代理 handler 在解析出 `user_id`、key 后，从 Key 行带出 `team_id` 写入。

**迁移策略：** 先 `ALTER TABLE ... ADD COLUMN team_id`，历史行 `NULL`；新代码对「仅个人」筛选 `WHERE team_id IS NULL`，团队视图 `WHERE team_id = ?`。

### 3.3 数据不变式（应用层保证）

- 每个 `teams` 至少一名 `role = owner` 的成员。移交或移除前须校验。
- `slug` 在实例内唯一；与 `name` 可分离展示。

---

## 4. 团队上下文（Team Context）

### 4.1 传递方式（推荐）

控制台请求在通过 `session_auth::resolve_console_session` 得到 `user_id` 之后，增加 **团队上下文**：

- **请求头：** `X-Team-Id: <numeric id>`（或 slug + 额外解析；首版建议数字 id 减少歧义）。
- **规则：** 若路由声明需要团队上下文，则必须同时满足：头存在、且 `(team_id, user_id)` 在 `team_members` 中存在。

未带头：视为 **个人上下文**（仅适用于「我的个人密钥」「个人审计」类接口）；团队级列表必须带头，否则 `400` 或 `422`（与 API 风格统一）。

### 4.2 可选：JWT Claims 扩展

将 `current_team_id` 写入 JWT 可减少每请求带头，但 **轮换团队需重新签发**，实现复杂度更高。**首版建议仅用 Header**，登录响应不强制含团队列表（可另 `GET /api/v1/me/teams`）。

### 4.3 辅助方法（Rust）

在 `src/` 新增小模块或 `session_auth` 扩展，例如：

```text
resolve_team_context(req, state, user_id) -> Result<Option<i64>, ApiError>
```

- 无 `X-Team-Id` → `Ok(None)`（个人）。
- 有则校验成员身份 → `Ok(Some(team_id))`，否则 `403` / `404`（与产品约定「不泄露团队存在」时统一用 404）。

---

## 5. 服务端模块与路由

### 5.1 建议文件布局

| 路径 | 职责 |
|------|------|
| `src/services/team.rs` | 创建团队、改名、删除、成员 CRUD、邀请签发与接受 |
| `src/services/repository.rs`（或拆 `team_repository.rs`） | SQL 集中，便于测试 |
| `src/handlers/team.rs` | HTTP 入参/出参、调用 service |
| `src/routes.rs` | 注册下表路由 |

与现网风格一致：控制台可改走 **`/api/v1/...`**，与 `me/api-keys` 并列。

### 5.2 路由草案（REST）

以下与 [交互文档 §10](../design/interaction/teams-and-members.md#十api-契约供后端联调草案) 对齐，路径前缀统一为 **`/api/v1`**：

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/teams` | 当前用户所属团队列表 |
| POST | `/teams` | 创建团队；body：`name`, `slug`；返回团队 + 自身 owner 成员 |
| GET | `/teams/{id}` | 详情（成员可见） |
| PATCH | `/teams/{id}` | Admin+；改 `name`/`slug` |
| DELETE | `/teams/{id}` | 仅 Owner；级联策略见 §3.2 |
| GET | `/teams/{id}/members` | 成员列表 |
| PATCH | `/teams/{id}/members/{user_id}` | Admin+；改角色（禁止越权赋 owner） |
| DELETE | `/teams/{id}/members/{user_id}` | Admin+；移除（Owner 约束见交互文档） |
| POST | `/teams/{id}/invitations` | Admin+；body：邮箱或用户名、`role` |
| DELETE | `/invitations/{id}` | 撤销；Admin+ |
| POST | `/invitations/accept` | body：`token`；登录态下接受 |

鉴权：均先 `resolve_console_session`；团队内操作再校验 `team_members.role` 矩阵（与交互文档 §6 一致）。

### 5.3 错误与安全

- 非成员访问团队资源：**404**（推荐）或 **403**（与现网统一即可）。
- 邀请 token：仅一次性使用；compare 使用常量时间比较哈希。
- 速率限制：邀请、接受接口可单独限流（配置项后续加）。

---

## 6. 与现有能力的衔接

### 6.1 `/api/v1/me/api-keys`

- **列表/创建：** 若带 `X-Team-Id`，则过滤/写入 `team_id`；不带则 `user_id` + `team_id IS NULL`。
- **PATCH/Revoke：** 校验 Key 行属于当前用户 **且** `team_id` 与头一致（或个人且空头）。
- **权限：** 团队 Key 的创建可限制为 Admin+（产品规则）；Member 只读列表则仅供查看自己可见范围（按交互定）。

### 6.2 审计 `/api/v1/logs/*`、`/analytics`

- 查询默认：个人上下文只查 `team_id IS NULL`；团队上下文查 `team_id = ?`。
- **越权：** 禁止仅传 query `team_id` 绕过成员校验；必须以 `X-Team-Id` + membership 为准。
- **导出：** 同上过滤条件写入导出任务元数据。

### 6.3 `POST /v1/chat/completions`

- 从 API Key 解析 `user_id` 与 Key 的 `team_id`；审计记录写入相同 `team_id`。
- 无需客户端传团队头（调用方只持 Key）。

### 6.4 内测路由 `POST /users`、`POST /users/{name}/keys`

- 保持 **实例管理/内测** 语义；是否支持指定 `team_id` 由运维需求决定，**默认不暴露**至公网控制台。

---

## 7. 前端实现要点

| 项 | 说明 |
|----|------|
| 状态 | 全局 store 或 React Context：`currentTeamId: number \| null`；与路由同步（可选把 `teamId` 放在 URL）。 |
| HTTP | axios/fetch 封装统一附加 `X-Team-Id`（当非空）。 |
| 路由 | 按 [交互文档 §4](../design/interaction/teams-and-members.md#四路由与页面)；与 `frontend/src/routes/index.tsx` 扩展。 |
| 布局 | `app-layout.tsx` 增加团队切换器；仅多团队或「有个人+团队」时显示完整下拉。 |
| 导航 | 更新 [导航设计](../design/interaction/navigation.md)「当前控制台实现」表。 |

---

## 8. 测试策略

| 层级 | 内容 |
|------|------|
| Rust 单元/集成 | team service：创建、邀请哈希、接受后成员存在、唯一 owner、slug 冲突 |
| Rust HTTP | `actix_web::test`：无成员 404、Admin 可邀请、Member 不可 |
| 前端单元 | 切换器与 header 注入（若抽取纯函数） |
| Playwright | 注册两用户 → 用户 A 建团队、邀请 B → B 接受 → B 可见团队 Key 列表；与 [AGENTS.md](../../AGENTS.md) 一致的可访问定位 |

E2E 数据：`e2e/config.toml` 可增第二测试用户，或单 spec 内 API 注册（若开放）。

---

## 9. 配置与运维

- 邀请链接 base URL：来自 `config.toml` 或前端 `origin`（邮件服务若后续接入）。
- 邀请有效期默认天数：可配置，默认 7 天。
- 备份：迁移后全库备份；删团队为硬删除时需确认审计保留策略。

---

## 10. 风险与缓解

| 风险 | 缓解 |
|------|------|
| 与 v1「单账户」文档混读 | 发布前更新 [实现状态](../implementation-status.md) 与 [产品范围与原则](../product/product-scope-principles.md)「若已上线团队则修订 v1 边界」一句 |
| `api_keys.team_id` 半上线 | 特性开关：仅当配置打开时控制台展示团队切换 |
| 性能 | `team_members` 上 `(user_id)` 索引，加速 `GET /teams` |

---

## 11. 交付检查清单

- [ ] `migrations/` 新文件 + `db.rs` 应用迁移（与现流程一致）  
- [ ] `src/routes.rs` + `development/api.md`  
- [ ] [实现状态](../implementation-status.md) 表格新增行  
- [ ] 前端路由 + E2E 新 spec 或扩展  
- [ ] `cargo test --all --all-features` 与 `npm run test:e2e`（变更触及前端时）  

---

**文档结束**
