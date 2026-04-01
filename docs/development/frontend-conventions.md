# ModelGate 前端开发约定

**版本:** 1.0  
**更新日期:** 2026年4月1日

---

## 一、目录与分层

- 使用 `feature-based` 目录组织：`features/<module>/{pages,components,hooks,types}`
- 通用组件放 `src/components/shared`
- 基础 UI 组件放 `src/components/ui`（shadcn）
- 全局状态放 `src/stores`
- 通用工具放 `src/lib`

---

## 二、命名规范

- 文件名：`kebab-case`（例如 `dashboard-page.tsx`）
- 组件名：`PascalCase`
- Hook：`useXxx`
- Zustand Store：`useXxxStore`
- Query Key：数组形式，第一位为资源名（如 `['logs', filters]`）

---

## 三、编码规范

- TypeScript `strict` 模式，禁止 `any`（必要时使用 `unknown`）
- 页面组件负责组装，不直接写请求逻辑
- 请求逻辑统一在 `lib/api-client.ts` 与 feature hooks
- 所有表单默认使用 Zod 做 schema 校验

---

## 四、提交与检查

- 提交前执行 `lint-staged`
- CI 必须通过：`lint`、`test`、`build`
- 提交信息使用 Conventional Commits
  - `feat(frontend): ...`
  - `fix(frontend): ...`
  - `chore(frontend): ...`

---

## 五、路由与权限

- 受保护页面必须通过 `AuthGuard`
- 管理员页面必须通过 `AdminGuard`
- 401 统一由 `api-client` 处理并跳转登录

---

**文档结束**
