# ModelGate 前端 E2E 测试方案

**版本:** 1.3  
**制定日期:** 2026年4月3日  
**更新说明:** 补充 API 密钥 E2E（`api-keys.spec.ts`）与 helpers  
**适用范围:** `frontend/` 管理控制台（React + Vite）+ 真实 `modelgate` 后端  
**关联:** [前端开发计划](frontend-development-plan.md) 3.10；[前端架构](../architecture/frontend-architecture.md) 测试章节；CI 工作流 [`.github/workflows/ci-e2e.yml`](../../.github/workflows/ci-e2e.yml)

---

## 一、目标与边界

### 1.1 目标

- 在真实浏览器中验证**关键用户路径**，在回归阶段尽早发现路由、鉴权、表单与 API 集成问题。
- **后端必须是真实的 Rust `modelgate` 进程**（SQLite、管理 API、代理转发、审计写入等走真实代码路径），与「只 Mock 前端 API」的测法区分。
- **上游 LLM 供应商（生产上常为 OpenRouter 等 OpenAI 兼容服务）在 E2E 中一律不访问公网**：由本机 **OpenAI 兼容 Mock** 承接，避免费用、配额与 flaky 外网。
- 与现有 **Vitest + Testing Library** 分工：Vitest 负责组件/Store/纯逻辑；E2E 负责跨页面、跨请求、持久化（如 `localStorage`）的闭环。

### 1.2 本期范围（P0）

与开发计划 3.10 对齐，优先覆盖：

| 场景 | 说明 |
|------|------|
| 注册 | `/register` → 邀请码 + 用户名/密码 → 成功提示或跳转登录 |
| 登录 | `/login` → 凭证 → 进入受保护布局（如 `/`） |
| 鉴权守卫 | 未登录访问 `/`、`/logs` 等 → 重定向登录并带 `redirect` |
| 日志中心 | `/logs` 列表加载、基础筛选或分页（随功能就绪扩展） |
| API 密钥管理 | 创建密钥、复制/吊销（页面落地后接入；路由 `/api-keys`；当前为占位页可暂缓） |

**Admin 路径**（`/channels`、`/users`、`/settings`）：在 `AdminGuard` 与对应页面可用后，用**独立 admin 账号**补充用例。

### 1.3 非目标（可后续迭代）

- 性能与可访问性的系统化基准（由 Lighthouse/专用工具补充）。
- 多浏览器全矩阵（首期以 Chromium 为主，必要时加 Firefox）。
- **对真实 OpenRouter / OpenAI 等公网上游的调用**（E2E 只验证经真实 Rust 代理到 **Mock 上游** 的链路；若需验供应商契约，用单独合约测试或预发环境）。

---

## 二、技术选型

**选用 [Playwright](https://playwright.dev/)**（与《前端架构》一致）。

| 考量 | 说明 |
|------|------|
| 稳定性 | 自动等待、网络/路由拦截、trace 与视频便于排障 |
| 与 Vite 协作 | `webServer` 启动 dev server；`baseURL` 指向 `http://127.0.0.1:3000` |
| CI | 官方 GitHub Actions 镜像、浏览器缓存成熟 |
| 团队 | TypeScript 一等公民，与前端栈一致 |

不选用 Cypress 的主要原因：本项目文档与架构已统一 Playwright；二选一即可，避免双栈。

---

## 三、环境与数据策略

### 3.1 真实 Rust + Mock OpenRouter（上游）

| 组件 | E2E 要求 |
|------|-----------|
| **modelgate（Rust）** | 使用与生产相同的二进制/构建产物启动，连接真实本地库文件、真实 HTTP 路由与代理逻辑。 |
| **OpenRouter（或任意 OpenAI 兼容上游）** | **不直连公网**。将配置中的 `upstream.base_url` 指向本机 Mock，使 Rust 仍执行完整 `POST` 转发与响应处理（含审计侧效果）。 |

**Mock 须实现的契约（与当前代理一致）**

- Rust 按 [`src/upstream.rs`](../../src/upstream.rs) 规则拼接 `.../v1/chat/completions`（`base_url` 以 `/v1` 结尾时最常见）。
- Mock 提供 **`POST /v1/chat/completions`**（或你配置的完整 base 所对应的单一路径），返回 **合法 OpenAI 式** JSON（非流式用例至少支持 `stream: false` 的 completion 结构；若 E2E 覆盖流式，再实现 **SSE** 分块响应）。
- `Authorization: Bearer ...` 可与生产一样传入；Mock 可忽略 token 或校验固定测试密钥。

**配置方式（与当前代码一致）**

- [`src/config.rs`](../../src/config.rs) 中：环境变量可覆盖 **`UPSTREAM_API_KEY`**、**`UPSTREAM_BASE_URL`**（指向 Mock 基址，如 `http://127.0.0.1:18080/v1`）、**`AUTH_INVITE_CODE`**；未设置时 `base_url` 来自 `config.toml` 或默认值。
- **推荐**：为 E2E 准备专用 `config.toml`（或独立工作目录下的配置文件），例如将：

  ```toml
  [upstream]
  base_url = "http://127.0.0.1:18080/v1"
  api_key = "e2e-dummy-upstream-key"
  ```

  其中 `18080` 为 Mock 监听端口；`api_key` 非空即可满足启动校验。生产中的 OpenRouter 基址（如 `https://openrouter.ai/api/v1`）仅用于非 E2E 环境。

**启动顺序**

1. 启动 **上游 Mock**（监听 `18080` 或约定端口）。  
2. 在持有上述 `config.toml` 的目录启动 **Rust**（监听 `8000`，并完成迁移）。  
3. 启动 **Vite**（`3000`）与 **Playwright**。

CI 中同样保持该顺序；可将 Mock 实现为小型 Node/Go/Rust 常驻服务，或使用 Wiremock 等工具挂载相同路径。

### 3.2 本地运行拓扑（控制台 ↔ 网关）

当前 `vite.config.ts` 将 `/api`、`/healthz` 等代理到 `http://127.0.0.1:8000`。在 **3.1** 已满足的前提下：

1. **Rust**：监听 `8000`（或统一改端口后同步改 Vite proxy）。  
2. **Vite**：由 Playwright `webServer` 启动 `npm run dev`（`3000`），或手动启动后运行 `npx playwright test`。

Playwright 配置中 `baseURL` 设为 `http://127.0.0.1:3000`，**不**强制设置 `VITE_API_BASE_URL`，以便走同源代理（与生产「静态站 + 同域 API」形态接近）。

**不建议**用 Playwright `page.route` 拦截「浏览器 → OpenRouter」来替代本方案：浏览器不直连 OpenRouter，流量经 **Rust 服务端** 发出；Mock 必须对 **Rust 进程** 可见。若仅 Mock 管理后台调用的 `/api/v1/...`，仍无法替代上游 Mock。

### 3.3 测试账号与邀请码

- **注册用例**依赖服务端配置的邀请码（见 [注册页交互](../design/interaction/register.md)）。通过环境变量注入，**禁止**把真实生产邀请码写入仓库。
  - 建议：`.env.e2e.local`（gitignore）或 CI Secrets：`E2E_INVITE_CODE`。
- **登录用例**需要已存在用户：
  - **推荐**：`globalSetup` 中调用后端 HTTP API（若已有「初始化用户/种子」接口则直接用；否则先 `register` 再 `login` 一次，将 storage state 写入 `playwright/.auth/user.json`）。
  - **备选**：手工在目标库中建用户，仅本地使用（文档中说明步骤）。

### 3.4 鉴权状态

应用使用 Zustand `persist`，存储键为 **`modelgate-auth`**（`localStorage`）。优先通过 **UI 登录**生成状态；仅在稳定化阶段对非登录用例使用 **`storageState`** 复用会话，减少用例时长。

---

## 四、项目结构约定

建议在 `frontend/` 下新增：

```text
frontend/
  e2e/
    fixtures/           # 可选：共享辅助函数
    mock-upstream/      # 可选：最小 OpenAI 兼容 Mock（或独立仓库脚本）
    auth.setup.ts       # 可选：登录并导出 storageState
    register.spec.ts
    login.spec.ts
    logs.spec.ts
    tokens.spec.ts      # 功能就绪后启用
  playwright.config.ts
```

npm scripts（落地时增加）：

- `test:e2e`：`playwright test`
- `test:e2e:ui`：`playwright test --ui`
- `test:e2e:headed`：本地调试

---

## 五、选择器与可测性

- **优先** `getByRole` / `getByLabel` / `getByPlaceholder`，与可访问性一致。
- 对复杂表格或重复控件，在关键节点增加 **`data-testid`**（与 [前端开发约定](frontend-conventions.md) 对齐，命名如 `logs-table`、`login-submit`）。
- 避免依赖文案的脆弱断言（除非产品强制要求）；日期/动态 ID 用部分匹配或专用 `testid`。

---

## 六、用例设计要点

### 6.1 注册（`register.spec.ts`）

- 前置：`E2E_INVITE_CODE` 已配置。
- 步骤：打开 `/register`，填写字段，提交。
- 断言：成功反馈或跳转 `/login`；错误时展示服务端/校验错误（与接口契约一致）。

### 6.2 登录与守卫（`login.spec.ts`）

- 使用种子用户登录 → 期望 URL 为 `/` 或 `redirect` 目标。
- 清除 `localStorage` 后访问 `/logs` → 期望跳转 `/login` 且 query 含 `redirect`。

### 6.3 日志中心（`logs.spec.ts`）

- 登录态下打开 `/logs`。
- 断言：表格或空状态出现；若有筛选 UI，覆盖「改条件 → URL query 同步」（与 [日志中心规格](../design/interaction/log-center.md) 一致时可加强）。

### 6.4 API 密钥（`frontend/e2e/api-keys.spec.ts`）

- **已登录（`describe` 内 `serial`）**：侧栏进入 `/api-keys`、直开路由、新建密钥（一次性明文 + `我已保存`）、列表掩码与行状态、剪贴板复制、`confirm` 吊销后 UI 与 **401 调用 chat**、与 `GET /api/v1/me/api-keys` 行数对齐。
- **未登录**：清空 `storageState` 访问 `/api-keys` → 跳转登录页。
- **辅助**：`e2e/helpers/api.ts` 增加 `listMyApiKeys` / `createMyApiKey` / `revokeMyApiKey`（经 Vite 代理打真实 Rust）。

### 6.5 Admin 用例（后续）

- 使用 `role: admin` 的用户执行 `storageState` 或在 setup 中分配角色（取决于后端种子能力）。
- 覆盖 `/channels` 等路由可访问；普通用户访问应被重定向到 `/`。

---

## 七、稳定性与排障

- `fullyParallel`：仅对无共享写状态的用例开启；登录类用例可用单 worker 或独立 project。
- 网络：**浏览器 → ModelGate（Rust）** 为真实 HTTP；**Rust → 上游** 在 E2E 中指向 Mock（见 3.1）。不要用 `page.route` 伪造 OpenRouter 响应来代替服务端 Mock，否则会绕过真实代理与审计路径。仅对「纯前端、且不依赖 Rust 上游」的边界可用 `page.route`，并单独标注语义。
- 失败产物：`trace: 'on-first-retry'`、`screenshot: 'only-on-failure'`，CI 上传 artifact。
- 重试：CI 上 `retries: 2`；本地默认 0 便于快速失败。

---

## 八、CI 建议

当前根目录 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) 仅执行 Rust。建议**新增**独立工作流（例如 `ci-e2e.yml`），避免拖慢每次 PR 的 Rust 门禁：

- **触发**：`paths` 包含 `frontend/**`、或对 `main` 的 nightly。
- **Job 步骤**：checkout → 安装 Node → `frontend` 下 `npm ci` → 安装 Playwright 浏览器 → **启动 OpenAI 兼容上游 Mock** → 使用 E2E 用 `config.toml` 启动 Rust（`sqlite.path` 指向 job 临时文件）→ 迁移并启动后端 → `npm run test:e2e`。
- **机密**：`E2E_INVITE_CODE`、测试用户密码等来自 GitHub Secrets。

若短期内无 CI 资源，可仅在文档中保留「本地必跑清单」，合并前由开发者执行 `npm run test:e2e`。

---

## 九、验收标准（方案落地时）

- Playwright 配置可在一台干净机器上按文档启动并通过 P0 用例。
- 至少 **注册、登录、未登录重定向、日志列表** 四条路径有自动化覆盖。
- `frontend/package.json` 提供统一脚本；README 或本文档第三节可复现运行方式。

---

## 十、实施顺序建议

1. 实现或引入 **最小 OpenAI 兼容上游 Mock**（固定端口、非流式响应可先跑通）。  
2. 增加 E2E 专用 **`config.toml`**（或工作目录约定），将 `upstream.base_url` 指向 Mock，保证 Rust 可启动且代理命中 Mock。  
3. 添加 `@playwright/test` 与 `playwright.config.ts`（`webServer` + `baseURL`），文档化「Mock → Rust → Vite → 测试」启动顺序。  
4. 约定 `E2E_*` 环境变量与本地 `.env.e2e.local` 示例（不提交密钥）。  
5. 实现 `login` + `AuthGuard` 用例。  
6. 接入 `register`（依赖邀请码）。  
7. 扩展 `logs`；若用例经 **API 密钥** 调用代理，确认审计/日志与 Mock 响应一致。API 密钥与 Admin 随功能交付增量补用例。  
8. 接入 CI 工作流与 artifact（含 Mock 进程生命周期）。

---

## 十一、参考

- [Playwright 文档](https://playwright.dev/docs/intro)  
- 路由与守卫：`frontend/src/routes/index.tsx`  
- API 与代理：`frontend/src/lib/api-client.ts`、`frontend/vite.config.ts`  
- 持久化键：`frontend/src/stores/auth-store.ts`（`modelgate-auth`）  
- 上游 URL 拼接：`src/upstream.rs`  
- 代理转发：`src/handlers/proxy.rs`  
- 配置与 `upstream` / `auth`：`src/config.rs`

---

## 十二、仓库内已落地（实现对照）

| 内容 | 位置 |
|------|------|
| OpenAI 兼容上游 Mock（非流式 + 简易 SSE） | `e2e/mock-openai-upstream.mjs` |
| Mock + `cargo run`（cwd=`e2e/`，读 `e2e/config.toml`） | `e2e/run-modelgate-stack.mjs` |
| E2E 后端配置（邀请码、Mock 基址、独立 sqlite） | `e2e/config.toml` |
| Playwright 与用例 | `frontend/playwright.config.ts`、`frontend/e2e/*.ts`（日志：`logs.spec.ts`；API 密钥：`api-keys.spec.ts`；辅助：`e2e/helpers/api.ts`） |
| 脚本 | `frontend`：`npm run test:e2e` / `test:e2e:ui` |
| CI | `.github/workflows/ci-e2e.yml` |
