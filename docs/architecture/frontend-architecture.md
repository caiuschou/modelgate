# ModelGate 前端架构设计方案

**版本:** 1.0
**编写日期:** 2026年4月1日
**技术栈:** React 19 + Vite 6 + shadcn/ui + TailwindCSS 4

> **模块落地：** 各 Feature 是否已实现见 [实现状态](../implementation-status.md)；HTTP 约定见 [开发 API](../development/api.md)。

---

## 一、架构概述

### 1.1 设计目标

ModelGate 前端作为管理控制台，为管理员、开发者和运营人员提供统一的可视化操作界面。核心设计目标：

- **高效开发** — 基于 shadcn/ui 组件库快速构建，避免重复造轮子
- **类型安全** — TypeScript 全覆盖，编译期发现问题
- **响应式** — 桌面端优先，兼顾平板和移动端
- **可维护** — 清晰的分层架构，模块边界明确
- **高性能** — Vite 构建，懒加载，最小化 bundle 体积

### 1.2 技术选型

| 分类 | 技术 | 版本 | 说明 |
|------|------|------|------|
| **构建工具** | Vite | 6.x | 极速 HMR，ESM 原生支持 |
| **UI 框架** | React | 19.x | Compiler 优化，无需手动 memo |
| **语言** | TypeScript | 5.x | 严格模式，类型安全 |
| **样式方案** | TailwindCSS | 4.x | 原子化 CSS，设计令牌映射 |
| **组件库** | shadcn/ui | latest | 可定制、可复制的组件，基于 Radix UI |
| **路由** | React Router | 7.x | 嵌套路由，数据加载 |
| **状态管理** | Zustand | 5.x | 轻量级，TypeScript 友好 |
| **数据请求** | TanStack Query | 5.x | 服务端状态管理，缓存、重试、轮询 |
| **HTTP 客户端** | ky | 1.x | 轻量 fetch 封装，拦截器、重试 |
| **表单** | React Hook Form + Zod | 7.x / 3.x | 高性能表单 + Schema 验证 |
| **图表** | Recharts | 2.x | 基于 D3 的 React 声明式图表 |
| **表格** | TanStack Table | 8.x | Headless 表格，排序、筛选、分页 |
| **国际化** | i18next | 24.x | 多语言支持，按需加载语言包 |
| **日期处理** | date-fns | 4.x | 函数式 API，Tree-shakable |
| **代码规范** | ESLint + Prettier | 9.x / 3.x | 统一代码风格 |
| **测试** | Vitest + Testing Library | 3.x / 16.x | 快速单测，组件测试 |

### 1.3 选型理由

**React + Vite 而非 Next.js：**
ModelGate 控制台是纯前端 SPA，后端 API 由 Rust (Actix-web) 提供。不需要 SSR/SSG，Vite 构建更轻量、HMR 更快。

**shadcn/ui 而非 Ant Design / Material UI：**
- shadcn/ui 组件代码直接复制到项目中，完全可控
- 基于 Radix UI 原语，无障碍性开箱即用
- TailwindCSS 原生集成，与设计令牌体系天然对齐
- 按需引入，不会引入整个组件库的体积

**Zustand 而非 Redux：**
- API 极简，样板代码极少
- 天然支持 TypeScript 类型推断
- 不需要 Provider 包裹
- 控制台应用状态规模不需要 Redux 的复杂度

**TanStack Query 而非 SWR：**
- 更完善的缓存策略（stale-while-revalidate、乐观更新）
- DevTools 支持
- 支持 Mutation、Infinite Query、Prefetch
- 与 Zustand 互补：Query 管理服务端状态，Zustand 管理客户端状态

---

## 二、项目结构

```
frontend/
├── index.html
├── package.json
├── tsconfig.json
├── tsconfig.app.json
├── tsconfig.node.json
├── vite.config.ts
├── tailwind.config.ts
├── components.json                    # shadcn/ui 配置
├── .env                               # 默认环境变量
├── .env.development                   # 开发环境变量
├── .env.production                    # 生产环境变量
│
├── public/
│   ├── favicon.svg
│   └── locales/                       # i18n 语言包
│       ├── zh-CN/
│       │   └── translation.json
│       └── en/
│           └── translation.json
│
└── src/
    ├── main.tsx                       # 应用入口
    ├── app.tsx                        # 根组件，路由 + Provider
    ├── vite-env.d.ts
    │
    ├── assets/                        # 静态资源
    │   └── logo.svg
    │
    ├── components/                    # 通用组件
    │   ├── ui/                        # shadcn/ui 基础组件（CLI 生成）
    │   │   ├── button.tsx
    │   │   ├── input.tsx
    │   │   ├── dialog.tsx
    │   │   ├── table.tsx
    │   │   ├── toast.tsx
    │   │   ├── dropdown-menu.tsx
    │   │   ├── command.tsx
    │   │   ├── sheet.tsx
    │   │   ├── badge.tsx
    │   │   ├── card.tsx
    │   │   ├── select.tsx
    │   │   ├── tabs.tsx
    │   │   ├── skeleton.tsx
    │   │   └── ...
    │   │
    │   ├── layout/                    # 布局组件
    │   │   ├── app-layout.tsx         # 主布局（侧边栏 + 顶栏 + 内容区）
    │   │   ├── sidebar.tsx            # 侧边导航
    │   │   ├── header.tsx             # 顶部栏
    │   │   ├── breadcrumb-nav.tsx     # 面包屑导航
    │   │   └── mobile-nav.tsx         # 移动端底部导航
    │   │
    │   └── shared/                    # 业务通用组件
    │       ├── data-table.tsx         # 通用数据表格（封装 TanStack Table）
    │       ├── stat-card.tsx          # 统计卡片
    │       ├── status-badge.tsx       # 状态标签
    │       ├── confirm-dialog.tsx     # 确认弹窗
    │       ├── search-command.tsx     # 全局搜索（Cmd+K）
    │       ├── date-range-picker.tsx  # 日期范围选择器
    │       ├── empty-state.tsx        # 空状态占位
    │       ├── error-boundary.tsx     # 错误边界
    │       ├── loading-skeleton.tsx   # 骨架屏
    │       └── copy-button.tsx        # 复制按钮（令牌复制等）
    │
    ├── features/                      # 功能模块（按业务划分）
    │   ├── dashboard/                 # 仪表盘
    │   │   ├── pages/
    │   │   │   └── dashboard-page.tsx
    │   │   └── components/
    │   │       ├── overview-cards.tsx
    │   │       ├── usage-trend-chart.tsx
    │   │       ├── model-ranking.tsx
    │   │       └── recent-errors.tsx
    │   │
    │   ├── channels/                  # 渠道管理
    │   │   ├── pages/
    │   │   │   ├── channel-list-page.tsx
    │   │   │   └── channel-detail-page.tsx
    │   │   ├── components/
    │   │   │   ├── channel-table.tsx
    │   │   │   ├── channel-form.tsx
    │   │   │   ├── channel-test-dialog.tsx
    │   │   │   └── channel-health-indicator.tsx
    │   │   ├── hooks/
    │   │   │   └── use-channels.ts
    │   │   └── types.ts
    │   │
    │   ├── tokens/                    # 令牌管理
    │   │   ├── pages/
    │   │   │   └── token-list-page.tsx
    │   │   ├── components/
    │   │   │   ├── token-table.tsx
    │   │   │   ├── token-create-dialog.tsx
    │   │   │   └── token-detail-sheet.tsx
    │   │   ├── hooks/
    │   │   │   └── use-tokens.ts
    │   │   └── types.ts
    │   │
    │   ├── users/                     # 用户管理
    │   │   ├── pages/
    │   │   │   └── user-list-page.tsx
    │   │   ├── components/
    │   │   │   ├── user-table.tsx
    │   │   │   └── user-form-dialog.tsx
    │   │   ├── hooks/
    │   │   │   └── use-users.ts
    │   │   └── types.ts
    │   │
    │   ├── logs/                      # 日志中心
    │   │   ├── pages/
    │   │   │   ├── log-list-page.tsx
    │   │   │   └── log-detail-page.tsx
    │   │   ├── components/
    │   │   │   ├── log-table.tsx
    │   │   │   ├── log-filters.tsx
    │   │   │   ├── log-detail-panel.tsx
    │   │   │   └── log-export-dialog.tsx
    │   │   ├── hooks/
    │   │   │   └── use-logs.ts
    │   │   └── types.ts
    │   │
    │   ├── analytics/                 # 统计分析
    │   │   ├── pages/
    │   │   │   └── analytics-page.tsx
    │   │   └── components/
    │   │       ├── usage-chart.tsx
    │   │       ├── cost-breakdown.tsx
    │   │       ├── model-distribution.tsx
    │   │       └── export-report-dialog.tsx
    │   │
    │   ├── auth/                      # 认证
    │   │   ├── pages/
    │   │   │   ├── login-page.tsx
    │   │   │   └── register-page.tsx
    │   │   ├── components/
    │   │   │   └── auth-form.tsx
    │   │   └── hooks/
    │   │       └── use-auth.ts
    │   │
    │   └── settings/                  # 系统设置
    │       ├── pages/
    │       │   └── settings-page.tsx
    │       └── components/
    │           ├── general-settings.tsx
    │           ├── model-pricing.tsx
    │           └── about-system.tsx
    │
    ├── hooks/                         # 全局自定义 Hooks
    │   ├── use-media-query.ts         # 响应式断点
    │   ├── use-debounce.ts            # 防抖
    │   ├── use-clipboard.ts           # 剪贴板
    │   ├── use-keyboard-shortcut.ts   # 快捷键
    │   └── use-theme.ts              # 主题切换
    │
    ├── lib/                           # 工具库
    │   ├── api-client.ts              # HTTP 客户端（ky 实例 + 拦截器）
    │   ├── query-client.ts            # TanStack Query 客户端配置
    │   ├── utils.ts                   # 工具函数（cn、format 等）
    │   ├── constants.ts               # 全局常量
    │   └── validators.ts              # Zod Schema 集合
    │
    ├── stores/                        # 客户端状态（Zustand）
    │   ├── auth-store.ts              # 认证状态（token、user）
    │   ├── ui-store.ts                # UI 状态（侧边栏折叠、主题）
    │   └── notification-store.ts      # 通知状态
    │
    ├── routes/                        # 路由定义
    │   └── index.tsx                  # 路由配置
    │
    ├── types/                         # 全局类型定义
    │   ├── api.ts                     # API 响应/请求类型
    │   └── common.ts                  # 通用类型
    │
    └── styles/
        └── globals.css                # TailwindCSS 入口 + CSS 变量
```

### 2.1 目录设计原则

**Feature-based 组织：** 按业务功能（dashboard、channels、tokens、logs 等）划分模块，每个模块内聚 pages、components、hooks、types，避免跨模块耦合。

**共享层分离：**
- `components/ui/` — shadcn/ui 原子组件，不包含业务逻辑
- `components/shared/` — 跨模块复用的业务组件
- `components/layout/` — 布局骨架组件
- `hooks/` — 全局通用 Hooks
- `lib/` — 工具函数和配置
- `stores/` — 全局客户端状态

**就近原则：** 仅当某个 hook / type / component 只被某一个 feature 使用时，放在 feature 目录内部；被两个以上 feature 使用时，提升到全局。

---

## 三、核心架构分层

```
┌──────────────────────────────────────────────────────────┐
│                      页面层 (Pages)                        │
│  feature/*/pages/ — 路由页面组件                           │
│  职责: 组装布局与功能组件，数据获取入口                      │
└──────────────────────────────────────────────────────────┘
                            │ 使用
                            ▼
┌──────────────────────────────────────────────────────────┐
│                    功能组件层 (Feature Components)          │
│  feature/*/components/ — 业务组件                          │
│  职责: 渲染 UI、处理交互、调用 hooks                        │
└──────────────────────────────────────────────────────────┘
                            │ 使用
                            ▼
┌──────────────────────────────────────────────────────────┐
│                     Hooks 层 (Data Hooks)                  │
│  feature/*/hooks/ + hooks/ — Query Hooks / 自定义逻辑       │
│  职责: 封装数据获取(TanStack Query)、业务逻辑               │
└──────────────────────────────────────────────────────────┘
                            │ 调用
                            ▼
┌──────────────────────────────────────────────────────────┐
│                     数据层 (API + Store)                    │
│  lib/api-client.ts — HTTP 请求                             │
│  stores/*.ts — 客户端状态(Zustand)                         │
│  职责: 与后端通信、管理客户端状态                             │
└──────────────────────────────────────────────────────────┘
                            │ 请求
                            ▼
┌──────────────────────────────────────────────────────────┐
│                   ModelGate Rust API                        │
│  /healthz · /v1/chat/completions · /api/v1/logs/*          │
│  /users · /users/{username}/keys · ...                     │
└──────────────────────────────────────────────────────────┘
```

### 3.1 数据流

```
用户操作
  │
  ▼
Page / Component
  │
  ├─ 读取数据 ──▶ useQuery (TanStack Query)
  │                  │
  │                  ▼
  │              api-client.ts ──▶ Rust API ──▶ 返回 JSON
  │                  │
  │                  ▼
  │              Query Cache (自动缓存、失效、重取)
  │
  ├─ 写入数据 ──▶ useMutation (TanStack Query)
  │                  │
  │                  ▼
  │              api-client.ts ──▶ Rust API
  │                  │
  │                  ▼
  │              invalidateQueries (自动刷新关联查询)
  │
  └─ 客户端状态 ──▶ Zustand Store (主题、侧边栏、通知)
```

### 3.2 请求拦截与错误处理

```typescript
// lib/api-client.ts 核心逻辑

import ky from "ky";

const apiClient = ky.create({
  prefixUrl: import.meta.env.VITE_API_BASE_URL,
  timeout: 30_000,
  hooks: {
    beforeRequest: [
      (request) => {
        const token = useAuthStore.getState().token;
        if (token) {
          request.headers.set("Authorization", `Bearer ${token}`);
        }
      },
    ],
    afterResponse: [
      async (_request, _options, response) => {
        if (response.status === 401) {
          useAuthStore.getState().logout();
          window.location.href = "/login";
        }
      },
    ],
  },
  retry: {
    limit: 2,
    methods: ["get"],
    statusCodes: [408, 502, 503, 504],
  },
});
```

---

## 四、路由设计

### 4.1 路由表

| 路径 | 页面 | 权限 | 说明 |
|------|------|------|------|
| `/login` | LoginPage | 公开 | 登录页 |
| `/register` | RegisterPage | 公开 | 注册页 |
| `/` | DashboardPage | 登录 | 仪表盘首页 |
| `/channels` | ChannelListPage | 管理员 | 渠道列表 |
| `/channels/:id` | ChannelDetailPage | 管理员 | 渠道详情 |
| `/tokens` | TokenListPage | 登录 | 令牌列表 |
| `/users` | UserListPage | 管理员 | 用户列表 |
| `/logs` | LogListPage | 登录 | 请求日志列表 |
| `/logs/:requestId` | LogDetailPage | 登录 | 日志详情 |
| `/analytics` | AnalyticsPage | 登录 | 统计分析 |
| `/settings` | SettingsPage | 管理员 | 系统设置 |
| `*` | NotFoundPage | 公开 | 404 页面 |

### 4.2 路由守卫

```typescript
// routes/index.tsx 核心结构

const router = createBrowserRouter([
  {
    path: "/login",
    element: <LoginPage />,
  },
  {
    path: "/",
    element: <AuthGuard><AppLayout /></AuthGuard>,
    children: [
      { index: true, element: <DashboardPage /> },
      { path: "channels", element: <AdminGuard><ChannelListPage /></AdminGuard> },
      { path: "channels/:id", element: <AdminGuard><ChannelDetailPage /></AdminGuard> },
      { path: "tokens", element: <TokenListPage /> },
      { path: "users", element: <AdminGuard><UserListPage /></AdminGuard> },
      { path: "logs", element: <LogListPage /> },
      { path: "logs/:requestId", element: <LogDetailPage /> },
      { path: "analytics", element: <AnalyticsPage /> },
      { path: "settings", element: <AdminGuard><SettingsPage /></AdminGuard> },
    ],
  },
  { path: "*", element: <NotFoundPage /> },
]);
```

### 4.3 懒加载

所有页面组件使用 `React.lazy` + `Suspense` 按路由分割代码：

```typescript
const DashboardPage = lazy(() => import("@/features/dashboard/pages/dashboard-page"));
const ChannelListPage = lazy(() => import("@/features/channels/pages/channel-list-page"));
const LogListPage = lazy(() => import("@/features/logs/pages/log-list-page"));
// ...
```

Vite 自动将每个 lazy import 拆分为独立 chunk，首屏仅加载当前路由所需代码。

---

## 五、状态管理策略

### 5.1 状态分类

| 状态类型 | 管理方式 | 示例 |
|---------|---------|------|
| **服务端状态** | TanStack Query | 渠道列表、令牌列表、日志数据、统计数据 |
| **客户端全局状态** | Zustand | 认证信息、主题、侧边栏折叠、通知 |
| **组件局部状态** | useState / useReducer | 表单值、弹窗开关、筛选条件 |
| **URL 状态** | React Router (searchParams) | 分页、排序、筛选参数 |

### 5.2 服务端状态 — TanStack Query

```typescript
// features/logs/hooks/use-logs.ts

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/lib/api-client";

interface LogFilters {
  startTime?: string;
  endTime?: string;
  model?: string;
  statusCode?: number;
  keyword?: string;
  /** 应用标识，键名与后端 OpenAPI 一致（如 `app` / `app_id`） */
  appId?: string;
  /** 多选时可序列化为重复键或逗号分隔，以后端为准 */
  finishReason?: string | string[];
  minPromptTokens?: number;
  maxPromptTokens?: number;
  minCompletionTokens?: number;
  maxCompletionTokens?: number;
  sort?: string;
  order?: "asc" | "desc";
  limit?: number;
  offset?: number;
}

export function useLogs(filters: LogFilters) {
  return useQuery({
    queryKey: ["logs", filters],
    queryFn: () =>
      apiClient.get("api/v1/logs/request", { searchParams: filters }).json(),
    staleTime: 30_000,
  });
}

export function useLogDetail(requestId: string) {
  return useQuery({
    queryKey: ["logs", requestId],
    queryFn: () =>
      apiClient.get(`api/v1/logs/request/${requestId}`).json(),
    enabled: !!requestId,
  });
}

export function useExportLogs() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: ExportParams) =>
      apiClient.post("api/v1/logs/export", { json: params }).json(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["exports"] });
    },
  });
}
```

### 5.3 客户端状态 — Zustand

```typescript
// stores/auth-store.ts

import { create } from "zustand";
import { persist } from "zustand/middleware";

interface AuthState {
  token: string | null;
  user: User | null;
  login: (token: string, user: User) => void;
  logout: () => void;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      token: null,
      user: null,
      login: (token, user) => set({ token, user }),
      logout: () => set({ token: null, user: null }),
    }),
    { name: "modelgate-auth" }
  )
);
```

```typescript
// stores/ui-store.ts

import { create } from "zustand";
import { persist } from "zustand/middleware";

type Theme = "light" | "dark" | "system";

interface UIState {
  sidebarCollapsed: boolean;
  theme: Theme;
  toggleSidebar: () => void;
  setTheme: (theme: Theme) => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      sidebarCollapsed: false,
      theme: "system",
      toggleSidebar: () =>
        set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
      setTheme: (theme) => set({ theme }),
    }),
    { name: "modelgate-ui" }
  )
);
```

---

## 六、样式体系

### 6.1 TailwindCSS 设计令牌映射

将 `docs/design/interaction/design-tokens.md` 中定义的设计令牌映射到 TailwindCSS CSS 变量体系，通过 shadcn/ui 的主题约定统一管理：

```css
/* src/styles/globals.css */

@import "tailwindcss";

@layer base {
  :root {
    /* 品牌色 */
    --primary: 210 100% 55%;           /* #1890FF */
    --primary-foreground: 0 0% 100%;
    --ring: 210 100% 55%;

    /* 功能色 */
    --destructive: 0 100% 65%;         /* #FF4D4F */
    --destructive-foreground: 0 0% 100%;

    /* 中性色 */
    --background: 0 0% 96%;            /* #F5F5F5 */
    --foreground: 0 0% 15%;            /* #262626 */
    --card: 0 0% 100%;                 /* #FFFFFF */
    --card-foreground: 0 0% 15%;
    --popover: 0 0% 100%;
    --popover-foreground: 0 0% 15%;
    --muted: 0 0% 94%;                 /* #F0F0F0 */
    --muted-foreground: 0 0% 35%;      /* #595959 */
    --border: 0 0% 85%;                /* #D9D9D9 */
    --input: 0 0% 85%;
    --accent: 210 100% 95%;            /* #E6F7FF */
    --accent-foreground: 210 100% 55%;

    /* 间距单位 */
    --radius: 0.25rem;                 /* 4px 基础圆角 */

    /* 侧边栏 */
    --sidebar-width: 200px;
    --sidebar-width-collapsed: 64px;
    --header-height: 56px;
  }

  .dark {
    --background: 0 0% 7%;
    --foreground: 0 0% 93%;
    --card: 0 0% 10%;
    --card-foreground: 0 0% 93%;
    --popover: 0 0% 10%;
    --popover-foreground: 0 0% 93%;
    --primary: 210 100% 55%;
    --primary-foreground: 0 0% 100%;
    --muted: 0 0% 15%;
    --muted-foreground: 0 0% 60%;
    --border: 0 0% 20%;
    --input: 0 0% 20%;
    --accent: 210 50% 15%;
    --accent-foreground: 210 100% 70%;
    --destructive: 0 70% 50%;
    --destructive-foreground: 0 0% 100%;
    --ring: 210 100% 55%;
  }
}
```

### 6.2 自定义语义色

在 `tailwind.config.ts` 中扩展功能色，对齐设计令牌中的渠道状态色和功能色：

```typescript
// tailwind.config.ts

import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        success: {
          DEFAULT: "#52C41A",
          bg: "#F6FFED",
          border: "#B7EB8F",
        },
        warning: {
          DEFAULT: "#FAAD14",
          bg: "#FFFBE6",
          border: "#FFE58F",
        },
        info: {
          DEFAULT: "#1890FF",
          bg: "#E6F7FF",
          border: "#91D5FF",
        },
      },
      fontFamily: {
        sans: [
          "-apple-system", "BlinkMacSystemFont", "Segoe UI", "Roboto",
          "Helvetica Neue", "Arial", "Noto Sans", "sans-serif",
        ],
        mono: [
          "SF Mono", "Monaco", "Inconsolata", "Fira Mono",
          "Droid Sans Mono", "Source Code Pro", "monospace",
        ],
      },
      fontSize: {
        "h1": ["24px", { lineHeight: "32px", fontWeight: "600" }],
        "h2": ["20px", { lineHeight: "28px", fontWeight: "600" }],
        "h3": ["16px", { lineHeight: "24px", fontWeight: "600" }],
        "body": ["14px", { lineHeight: "22px", fontWeight: "400" }],
        "body-sm": ["12px", { lineHeight: "20px", fontWeight: "400" }],
        "caption": ["12px", { lineHeight: "20px", fontWeight: "400" }],
        "overline": ["10px", { lineHeight: "16px", fontWeight: "400" }],
      },
      spacing: {
        "xs": "4px",
        "sm-space": "8px",
        "md-space": "12px",
        "lg-space": "16px",
        "xl-space": "24px",
        "xxl": "32px",
        "xxxl": "48px",
      },
      boxShadow: {
        "card": "0 1px 2px rgba(0,0,0,0.03)",
        "card-hover": "0 4px 12px rgba(0,0,0,0.08)",
        "dropdown": "0 6px 16px rgba(0,0,0,0.08)",
        "modal": "0 12px 24px rgba(0,0,0,0.12)",
      },
      animation: {
        "slide-in-left": "slideInLeft 300ms ease-out",
        "slide-in-top": "slideInTop 250ms ease-out",
        "fade-in": "fadeIn 200ms ease-out",
      },
    },
  },
} satisfies Config;
```

### 6.3 响应式断点

对齐交互设计文档的断点定义，TailwindCSS 默认断点基本匹配：

| 设计断点 | TailwindCSS 断点 | 宽度 | 适配设备 |
|---------|-----------------|------|---------|
| xs | 默认（无前缀） | < 640px | 手机 |
| sm | `sm:` | ≥ 640px | 大手机 |
| md | `md:` | ≥ 768px | 平板 |
| lg | `lg:` | ≥ 1024px | 桌面 |
| xl | `xl:` | ≥ 1280px | 大桌面 |

**布局适配规则：**

| 断点 | 侧边栏 | 表格 | 卡片网格 |
|------|--------|------|---------|
| 默认 | 隐藏，Sheet 抽屉 | 横向滚动 | 1 列 |
| sm | 隐藏，Sheet 抽屉 | 横向滚动 | 1 列 |
| md | 可折叠（图标模式） | 部分列隐藏 | 2 列 |
| lg | 固定显示 | 完整显示 | 3 列 |
| xl | 固定显示 | 完整显示 | 4 列 |

---

## 七、组件设计

### 7.1 布局组件

**AppLayout** — 主布局骨架：

```
┌────────────────────────────────────────────────────────┐
│ Header (固定，h-14)                                     │
│ [Logo] ModelGate    [Cmd+K 搜索]    [通知] [用户头像 ▼] │
├──────────┬─────────────────────────────────────────────┤
│ Sidebar  │ Main Content                                │
│ (w-[200] │ ┌───────────────────────────────────────┐  │
│ 或 w-16) │ │ Breadcrumb                            │  │
│          │ ├───────────────────────────────────────┤  │
│ 🏠 首页  │ │                                       │  │
│ 🔌 渠道  │ │          Page Content                 │  │
│ 🔑 令牌  │ │          (<Outlet />)                 │  │
│ 👥 用户  │ │                                       │  │
│ 📊 统计  │ │                                       │  │
│ 📝 日志  │ │                                       │  │
│ ⚙️ 设置  │ │                                       │  │
│          │ └───────────────────────────────────────┘  │
├──────────┴─────────────────────────────────────────────┤
│ (移动端) 底部导航 [首页] [统计] [令牌] [我的]            │
└────────────────────────────────────────────────────────┘
```

### 7.2 通用数据表格

封装 TanStack Table + shadcn/ui Table，提供统一的表格交互：

**功能：**
- 列排序（点击表头）
- 列筛选（下拉/输入）
- 分页（服务端分页）
- 行选择（批量操作）
- 列可见性切换
- 空状态 / 加载态 / 错误态
- 移动端卡片视图自动切换

```typescript
// components/shared/data-table.tsx 使用示例

<DataTable
  columns={columns}
  data={logs}
  loading={isLoading}
  pagination={{
    pageIndex,
    pageSize,
    total: data?.total ?? 0,
    onPageChange: setPageIndex,
    onPageSizeChange: setPageSize,
  }}
  emptyMessage="暂无请求日志"
/>
```

### 7.3 全局搜索

实现 `Cmd+K` / `Ctrl+K` 全局搜索，使用 shadcn/ui `Command` 组件：

**功能：**
- 页面导航搜索
- 快捷操作（新增渠道、新增令牌等）
- 防抖 300ms
- 最多显示 10 条结果
- 键盘导航

### 7.4 shadcn/ui 组件清单

项目所需的 shadcn/ui 组件：

| 组件 | 用途 |
|------|------|
| Button | 主按钮、次按钮、危险按钮、图标按钮 |
| Input / Textarea | 表单输入 |
| Select | 下拉选择（模型、渠道类型等） |
| Dialog | 创建/编辑弹窗 |
| Sheet | 移动端侧边抽屉、详情面板 |
| Table | 数据表格 |
| Card | 统计卡片、信息卡片 |
| Badge | 状态标签（正常/故障/禁用） |
| Tabs | 页面内标签切换 |
| Command | 全局搜索 (Cmd+K) |
| DropdownMenu | 用户菜单、操作菜单 |
| Toast / Sonner | 操作反馈通知 |
| Skeleton | 骨架屏加载 |
| AlertDialog | 危险操作确认 |
| Tooltip | 折叠侧边栏图标提示 |
| Popover | 筛选面板、日期选择 |
| Switch | 开关（渠道启用/禁用） |
| Separator | 分割线 |
| ScrollArea | 内容滚动区域 |
| Avatar | 用户头像 |

---

## 八、API 对接

### 8.1 当前后端 API 清单

基于 `src/routes.rs` 已实现的接口：

| 方法 | 路径 | 用途 | 前端模块 |
|------|------|------|---------|
| GET | `/healthz` | 健康检查 | 系统状态 |
| POST | `/users` | 创建用户 | 用户管理 |
| POST | `/users/{username}/keys` | 创建 API Key | 令牌管理 |
| POST | `/v1/chat/completions` | 聊天代理 | （不直接使用） |
| GET | `/api/v1/logs/request` | 日志列表 | 日志中心 |
| GET | `/api/v1/logs/request/{request_id}` | 日志详情 | 日志中心 |
| POST | `/api/v1/logs/export` | 导出日志 | 日志中心 |
| GET | `/api/v1/logs/export/{export_id}` | 导出状态 | 日志中心 |
| GET | `/api/v1/logs/export/{export_id}/download` | 下载导出文件 | 日志中心 |

### 8.2 前端需要的扩展 API

前端完整功能还需后端补充以下管理接口：

| 方法 | 路径 | 用途 | 优先级 |
|------|------|------|--------|
| POST | `/api/v1/auth/login` | 用户登录 | P0 |
| POST | `/api/v1/auth/logout` | 退出登录 | P0 |
| GET | `/api/v1/auth/me` | 获取当前用户信息 | P0 |
| GET | `/api/v1/users` | 用户列表 | P0 |
| PUT | `/api/v1/users/{id}` | 更新用户 | P1 |
| DELETE | `/api/v1/users/{id}` | 删除用户 | P1 |
| GET | `/api/v1/tokens` | 令牌列表 | P0 |
| PUT | `/api/v1/tokens/{id}` | 更新令牌 | P1 |
| DELETE | `/api/v1/tokens/{id}` | 删除/吊销令牌 | P0 |
| GET | `/api/v1/channels` | 渠道列表 | P0 |
| POST | `/api/v1/channels` | 创建渠道 | P0 |
| PUT | `/api/v1/channels/{id}` | 更新渠道 | P0 |
| DELETE | `/api/v1/channels/{id}` | 删除渠道 | P1 |
| POST | `/api/v1/channels/{id}/test` | 测试渠道连通性 | P1 |
| GET | `/api/v1/stats/overview` | 仪表盘概览数据 | P0 |
| GET | `/api/v1/stats/usage` | 用量趋势数据 | P1 |
| GET | `/api/v1/stats/models` | 模型调用统计 | P1 |

### 8.3 API 类型定义

```typescript
// types/api.ts

interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

interface PaginatedResponse<T> {
  items: T[];
  total: number;
  limit: number;
  offset: number;
}

interface AuditLog {
  request_id: string;
  user_id: string;
  token_id: string;
  channel_id: string;
  model: string;
  status_code: number;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  cost: number;
  latency_ms: number;
  created_at: string;
}

interface User {
  id: number;
  username: string;
  role: "admin" | "user";
  status: "active" | "suspended";
  created_at: string;
}

interface Token {
  id: number;
  user_id: number;
  name: string;
  key_prefix: string;
  status: "active" | "revoked" | "expired";
  quota_limit: number;
  quota_used: number;
  expires_at: string | null;
  created_at: string;
}

interface Channel {
  id: number;
  name: string;
  provider: string;
  base_url: string;
  models: string[];
  priority: number;
  weight: number;
  status: "active" | "disabled" | "error";
  health_status: "healthy" | "degraded" | "unhealthy";
  created_at: string;
}
```

---

## 九、前端与后端集成

### 9.1 开发环境代理

通过 Vite dev server 代理 API 请求到本地 Rust 后端：

```typescript
// vite.config.ts

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    port: 3000,
    proxy: {
      "/api": {
        target: "http://127.0.0.1:8000",
        changeOrigin: true,
      },
      "/healthz": {
        target: "http://127.0.0.1:8000",
        changeOrigin: true,
      },
      "/users": {
        target: "http://127.0.0.1:8000",
        changeOrigin: true,
      },
    },
  },
});
```

### 9.2 生产环境部署

生产环境下前端构建产物为纯静态文件，有两种部署方式：

**方案 A：Rust 后端托管静态文件（推荐初期）**

Actix-web 使用 `actix-files` 托管前端 `dist/` 目录，单端口同时服务 API 和前端：

```
browser ──▶ 165.22.55.30:8000
               │
               ├── /api/*        → Actix handler（API）
               ├── /healthz      → Actix handler（健康检查）
               └── /*            → actix-files（前端 SPA）
```

**方案 B：Nginx 反向代理（推荐规模化后）**

```
browser ──▶ Nginx (:80/:443)
               │
               ├── /api/*        → proxy_pass http://127.0.0.1:8000
               └── /*            → root /opt/modelgate/frontend/dist
```

### 9.3 环境变量

```bash
# .env.development
VITE_API_BASE_URL=http://localhost:3000

# .env.production
VITE_API_BASE_URL=
```

生产环境 `VITE_API_BASE_URL` 留空，使用相对路径，由同域服务或 Nginx 代理自动处理。

---

## 十、性能优化

### 10.1 构建优化

| 策略 | 实现方式 |
|------|---------|
| **代码分割** | React.lazy 按路由分割 + Vite 自动 chunk |
| **Tree Shaking** | ESM + Vite 默认支持 |
| **CSS 压缩** | TailwindCSS 自动 purge 未使用的 class |
| **资源压缩** | Vite 生产构建自动 minify JS/CSS |
| **Gzip/Brotli** | Nginx 或 `vite-plugin-compression` |
| **依赖预构建** | Vite `optimizeDeps` 预构建 node_modules |

### 10.2 运行时优化

| 策略 | 实现方式 |
|------|---------|
| **查询缓存** | TanStack Query staleTime / gcTime 控制 |
| **防抖搜索** | useDebounce hook，300ms |
| **虚拟滚动** | 大列表使用 @tanstack/react-virtual |
| **图片懒加载** | 原生 `loading="lazy"` |
| **骨架屏** | Skeleton 组件，避免布局抖动 |
| **乐观更新** | useMutation onMutate 中预更新缓存 |

### 10.3 Bundle 体积控制

```
预期 bundle 分布：

vendor.js     ~150KB  (React, React DOM, React Router)
ui.js          ~80KB  (Radix UI primitives, class-variance-authority)
query.js       ~30KB  (TanStack Query)
charts.js      ~60KB  (Recharts, D3 子集)  — 仅 dashboard/analytics 路由加载
i18n.js        ~15KB  (i18next core)
app.js         ~50KB  (业务代码)
────────────────────
首屏总计       ~325KB (gzip 后约 95KB)
```

---

## 十一、开发规范

### 11.1 命名规范

| 类型 | 规范 | 示例 |
|------|------|------|
| 文件名 | kebab-case | `channel-list-page.tsx` |
| 组件名 | PascalCase | `ChannelListPage` |
| Hook 名 | camelCase, `use` 前缀 | `useChannels` |
| Store 名 | camelCase, `use...Store` | `useAuthStore` |
| 常量 | UPPER_SNAKE_CASE | `API_BASE_URL` |
| 类型/接口 | PascalCase | `AuditLog`, `Channel` |
| CSS 变量 | kebab-case, `--` 前缀 | `--primary` |
| Query Key | 数组，第一项为实体名 | `["logs", filters]` |

### 11.2 组件编写规范

```typescript
// 标准组件结构

import { type ComponentProps } from "react";
import { cn } from "@/lib/utils";

interface StatCardProps extends ComponentProps<"div"> {
  title: string;
  value: string | number;
  trend?: { value: number; direction: "up" | "down" };
  icon?: React.ReactNode;
}

export function StatCard({ title, value, trend, icon, className, ...props }: StatCardProps) {
  return (
    <div className={cn("rounded-lg border bg-card p-4 shadow-card", className)} {...props}>
      <div className="flex items-center justify-between">
        <p className="text-body-sm text-muted-foreground">{title}</p>
        {icon}
      </div>
      <p className="mt-2 text-h2">{value}</p>
      {trend && (
        <p className={cn("mt-1 text-body-sm", trend.direction === "up" ? "text-success" : "text-destructive")}>
          {trend.direction === "up" ? "↑" : "↓"} {trend.value}%
        </p>
      )}
    </div>
  );
}
```

### 11.3 Git 分支与提交

| 分支 | 用途 |
|------|------|
| `main` | 生产分支 |
| `develop` | 开发集成分支 |
| `feat/xxx` | 功能分支 |
| `fix/xxx` | 修复分支 |

提交信息遵循 Conventional Commits：
- `feat(frontend): add channel management page`
- `fix(frontend): fix sidebar collapse animation`
- `chore(frontend): upgrade shadcn/ui components`

---

## 十二、测试策略

### 12.1 测试分层

| 层次 | 工具 | 覆盖目标 | 比例 |
|------|------|---------|------|
| 单元测试 | Vitest | 工具函数、Store、数据转换 | 60% |
| 组件测试 | Vitest + Testing Library | 组件渲染、交互、状态变化 | 30% |
| E2E 测试 | Playwright | 关键业务流程（登录、创建令牌、查看日志） | 10% |

### 12.2 关键测试场景

- 登录 / 登出 / Token 过期自动跳转
- 创建令牌并复制 Key
- 日志列表筛选、分页、详情查看
- 渠道创建和连通性测试
- 仪表盘数据加载和图表渲染
- 响应式布局：移动端侧边栏抽屉切换
- 错误边界：API 报错时的降级展示

---

## 十三、迭代计划

### 13.1 Phase 1 — 基础骨架（Week 1-2）

- [x] Vite + React + TypeScript 项目初始化
- [ ] TailwindCSS + shadcn/ui 配置
- [ ] 主布局（侧边栏、顶栏、内容区）
- [ ] 路由结构搭建
- [ ] API 客户端 + TanStack Query 配置
- [ ] 登录页 + Auth Store + 路由守卫
- [ ] 主题切换（明/暗/系统）

### 13.2 Phase 2 — 核心页面（Week 3-4）

- [ ] 仪表盘：概览卡片 + 用量趋势图
- [ ] 日志中心：列表 + 筛选 + 详情 + 导出
- [ ] 令牌管理：列表 + 创建 + 复制 + 吊销
- [ ] 用户管理：列表 + 创建

### 13.3 Phase 3 — 完善功能（Week 5-6）

- [ ] 渠道管理：列表 + 创建 + 编辑 + 测试
- [ ] 统计分析：图表 + 报表导出
- [ ] 全局搜索 (Cmd+K)
- [ ] 通知中心
- [ ] 系统设置页

### 13.4 Phase 4 — 打磨体验（Week 7-8）

- [ ] 移动端适配
- [ ] 国际化（中/英）
- [ ] 骨架屏 + 空状态 + 错误边界
- [ ] 性能优化
- [ ] E2E 测试
- [ ] 生产部署集成

---

## 十四、初始化命令

```bash
# 创建项目
npm create vite@latest frontend -- --template react-ts
cd frontend

# 安装核心依赖
npm install react-router-dom @tanstack/react-query zustand ky
npm install react-hook-form @hookform/resolvers zod
npm install recharts @tanstack/react-table
npm install date-fns i18next react-i18next
npm install class-variance-authority clsx tailwind-merge
npm install lucide-react

# 安装开发依赖
npm install -D tailwindcss @tailwindcss/vite
npm install -D @types/node
npm install -D vitest @testing-library/react @testing-library/jest-dom

# 初始化 shadcn/ui
npx shadcn@latest init

# 安装常用 shadcn/ui 组件
npx shadcn@latest add button input dialog table card badge
npx shadcn@latest add select tabs command dropdown-menu toast
npx shadcn@latest add sheet skeleton tooltip popover switch
npx shadcn@latest add separator scroll-area avatar alert-dialog
npx shadcn@latest add sonner
```

---

**文档结束**
