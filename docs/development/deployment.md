# ModelGate 部署与运行

**版本:** 1.0  
**更新日期:** 2026年4月3日  

本文档描述本仓库 **Rust 网关 + 可选前端控制台** 的本地与简易生产部署要点。

---

## 一、依赖

- **Rust：** stable（见 `rust-toolchain.toml` 或 CI 配置）  
- **操作系统：** Windows / Linux / macOS  
- **数据库：** 内置 **SQLite**（路径由配置指定，自动迁移）

---

## 二、配置文件

1. 复制示例配置：

   ```bash
   cp config.example.toml config.toml
   ```

2. 编辑 `config.toml`：

   - **`[upstream]`**  
     - `base_url`：上游 OpenAI 兼容 API 根路径（默认 `https://api.openai.com/v1`）。  
     - `api_key`：可留占位，**生产推荐用环境变量** `UPSTREAM_API_KEY` 注入真实密钥。  
   - **`[auth].invite_code`**：自助注册邀请码；设为 `""` 关闭注册。可用环境变量 `AUTH_INVITE_CODE` 覆盖。  
   - **`[server]`**：监听地址与端口（默认 `0.0.0.0:8000`）。

3. **环境变量（常用）**

   | 变量 | 作用 |
   |------|------|
   | `UPSTREAM_API_KEY` | 覆盖上游 API Key（优先于配置文件） |
   | `UPSTREAM_BASE_URL` | 覆盖上游 Base URL |
   | `AUTH_INVITE_CODE` | 覆盖邀请码 |
   | `OPENAI_ORGANIZATION` | 可选，转发给上游 |
   | `OPENAI_PROJECT` | 可选，转发给上游 |

4. **审计与导出目录**（默认值见 `src/config.rs`）

   - `audit.log_dir`：请求/响应体落盘目录  
   - `audit.export_dir`：导出文件目录  
   - 确保进程用户对该路径有读写权限。

---

## 三、启动后端

在项目根目录：

```bash
cargo run
```

或使用 release：

```bash
cargo build --release
./target/release/modelgate
```

服务启动后会监听 `config.toml` 中的 `[server]` 地址。

### 3.1 验证

```bash
curl -s http://127.0.0.1:8000/healthz
```

### 3.2 调用 Chat（需先登录或 `POST /users` 取得 Key）

```bash
curl http://127.0.0.1:8000/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"gpt-4.1-mini\",\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}]}"
```

---

## 四、启动前端控制台（可选）

目录：`frontend/`

```bash
cd frontend
npm ci
npm run dev
```

默认 Vite：`http://127.0.0.1:3000`。  
开发环境下，`/api`、`/healthz`、`/users` 会代理到 `http://127.0.0.1:8000`（见 `vite.config.ts`）。

生产构建：

```bash
npm run build
npm run preview
```

将 `dist/` 静态资源挂到 Nginx/Caddy 等，**反向代理** `/api`、`/users`（及如需在同源下调用的其它后端路径）到网关进程。

---

## 五、生产部署注意

1. **管理接口：** `POST /users`、`POST /users/{username}/keys` 当前无应用层管理员鉴权，勿对公网暴露。  
2. **HTTPS：** 在反向代理终止 TLS，后端可仅监听内网。  
3. **密钥：** 勿将真实 `UPSTREAM_API_KEY` 提交到仓库；使用环境变量或密钥管理。  
4. **SQLite 路径：** 将 `sqlite.path` 指向持久卷，并做好备份。  
5. **CORS：** 按需改为白名单域名，而非 permissive。

---

## 六、相关文档

- [服务端 API（当前实现）](api.md)  
- [实现状态与文档对照](../implementation-status.md)  
- [前端部署方案](frontend-deployment-plan.md)  

---

**文档结束**
