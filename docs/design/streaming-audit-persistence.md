# 流式响应审计持久化与详情「不截断」展示方案

**版本:** 1.0  
**编写日期:** 2026年4月7日  
**状态:** 已部分落地（流式落盘 + DB 二阶段更新 + 读 body 不限 5MiB + 详情虚拟列表）；下载/纯流式 API 见分期  
**关联:** [审计日志技术方案](../architecture/audit-log-technical-solution.md) · [日志中心交互](../design/interaction/log-center.md) · `src/handlers/proxy.rs` · `src/audit.rs`

---

## 1. 背景与目标

### 1.1 现状

- **非流式** `chat/completions`：请求体、完整 JSON 响应体会落盘，审计表中有 `request_body_path` / `response_body_path`；控制台详情可通过 `GET /api/v1/logs/request/{id}/body` 查看正文。
- **流式**（`stream: true`）：在拿到上游 HTTP 状态后即写入一条审计记录，`**response_body_path` 为空**；边转发 SSE 边仅用已有逻辑从流中解析用量；**响应正文不落盘**。

结果是：流式调用的详情页无法像非流式一样查看完整响应轨迹。

### 1.2 目标

1. **流式响应与磁盘**：将上游返回的 **原始 SSE（`text/event-stream`）字节流**完整、顺序持久化到审计目录（不在存储阶段人为截断）。
2. **审计行与列表体验**：在流式会话 **正常结束或异常结束** 后，为同一条 `request_id` **补齐** `response_body_path`，并尽可能补齐与非流式一致的用量字段（`prompt_tokens`、`completion_tokens`、`total_tokens`、`finish_reason` 等）及 **端到端 `latency_ms`**。列表中「无响应路径」的空洞应消失。
3. **控制台详情「不截断」**（产品约束）：
  - **存储与传输**：不在业务上规定「最多 N 字符/字节」；服务端不因「过大」拒绝读写审计文件（需与运维上限、反代配置另行约定）。
  - **界面**：不在前端用 `slice`、省略号等方式**故意截断**正文；用户应能通过页面滚动浏览**全文**。极大响应通过 **虚拟化列表 / 只读大文档编辑器组件**等技术避免单次巨型 DOM，而不是用「截断文案」冒充全文。

> 说明：「不在产品层截断」不等于「浏览器进程内存无限」；超大内容仍受用户设备与单次请求传输实际限制，方案通过**流式下载 + 分块渲染**降低风险，见第 6 节。

---

## 2. 持久化格式

**默认：原始 SSE 文本**

- 与客户端收到的字节序列一致（含 `data:` 行、空行、`[DONE]` 等，以实际供应商为准）。
- 文件命名与目录规则与现有一致：`audit_config.log_dir` 下按月桶目录 + `{request_id}-response.json` **或** 为区分语义使用 `{request_id}-response.sse`（需与现有 `save_body_to_file` 的 `body_type` 参数扩展约定二选一，避免破坏已有工具链）。
- `metadata` 中增加或保留明确标记，例如：`"stream": true`、`"response_body_format": "text/event-stream"`，供详情 UI 选择展示模式（纯文本 / 可选将来 SSE 分行视图）。

**不在首版范围**

- 将 SSE 增量「合并」为单一 Chat Completions JSON（与供应商格式强耦合，单独迭代）。

---

## 3. 后端设计

### 3.1 写入时机与并发模型

- **请求体**：沿用现有逻辑（进入代理时已 `save_body_to_file`）。
- **响应体（流式）**：
  - 在上游返回可建立流之后，创建响应文件（可 `create` + 按 chunk **异步追加写入**），避免整段响应进入内存。
  - 在 `bytes_stream` 循环中：对每个 `Ok(chunk)`，除 **照旧 `yield` 给客户端** 外，向该文件 **append**（注意背压：写入慢时不拖死转发，可采用带界通道或 `tokio::io::AsyncWriteExt` + 适当 buffer）。
- **流结束**（`Stream` 自然结束）：`flush` 文件，解析缓冲区中剩余 SSE 行以更新用量（与现有 `flush_sse_usage_tail` 一致），然后 **更新数据库审计行**。

### 3.2 审计数据库：两段式更新

- **第一段（与今类似）**：流开始前或拿到状态码后 **插入**（或 `INSERT OR REPLACE`）一行，保证列表尽快可见；`response_body_path` 可先为空；`latency_ms` 可表示「至首包/至建立流」或暂占位，**产品需在文档中定义**最终对外含义以免误解。
- **第二段（新增）**：流结束后 **UPDATE** 同一 `request_id`（或再次 `REPLACE` 全行，注意保留 `created_at` 语义）：写入 `response_body_path`、刷新 `prompt_tokens` / `completion_tokens` / `total_tokens` / `finish_reason` / `cost`（若有）、**完整请求 `latency_ms`**，并可合并 `metadata`（如 `stream_completed: true` / `stream_aborted: true`）。

需新增或复用 `db` 层：`update_audit_log_stream_completion(...)`（事务内一次性更新，避免半更新）。

### 3.3 异常与客户端断开

- **上游读失败**：停止写入，文件可保留已写入片段；审计行 `UPDATE` 时写入 `response_body_path` + `error_message` 或 `metadata.stream_aborted`。
- **客户端中途断开**：Actix 侧流可能取消；应在 `Drop` 或显式 `scopeguard` 中 **关闭文件并仍尝试 UPDATE**（标记中止），保证磁盘上为**已产生部分的完整副本**（不对内容截断，只标记「未正常结束」）。

### 3.4 路径存储与安全

- 仍写入与现网一致的「相对工作目录或规范化路径」字符串；`read_audit_body_bytes` 已支持「相对 log_dir」与「含 log_dir 前缀的相对 cwd」两套路径（见 `src/audit.rs`），流式文件沿用同一套读取与 **路径逃逸校验**。

---

## 4. API 与「不截断」

### 4.1 当前缺口

- `GET .../body` 存在 `**MAX_AUDIT_BODY_BYTES`（如 5 MiB）**，超过则拒绝；与「不在业务层截断」冲突。

### 4.2 建议演进

1. **取消「读盘硬性上限」作为拒绝条件**，或改为仅打日志 / Metrics；**读盘**仍受 OS 与部署内存约束。
2. 对极大对象，优先 **分块传输（chunked）** 或专用 `**GET .../body/download`**（`Content-Disposition: attachment`），与详情内嵌 **同一权限模型**（同会话、同 `request_id` 归属校验）。
3. 反代与大文件：调大 `**proxy_read_timeout`**、合理关闭或调大 body buffer，避免在网关层静默截断。

详情页若继续用 `ky`/fetch 一次读入 `text()`，极大时仍会占用浏览器内存；因此 **UI 层应采用分块读取 + 增量交给虚拟化组件**（见第 6 节），与「API 不拒绝对象」配套。

---

## 5. 前端（控制台）

### 5.1 数据刷新

- 用户若在 **流尚未结束** 时打开详情：初始无 `response_body_path`；流结束后 **行被 UPDATE**，前端需 **自动 refetch 详情**（短轮询、或详情接口支持 `ETag`/`updated_at`、或 WebSocket 后续迭代）才能显示响应卡片。MVP 可采用「手动刷新」+ 文档说明，但最终应减少困惑。

### 5.2 展示组件（满足「不截断」且不死浏览器）

- **禁止**：`content.slice(0, N)` +「以下内容已省略」作为默认策略。
- **推荐**：
  - 只读 **CodeMirror 6 / Monaco** 等大文档模式，或
  - **虚拟列表按行渲染** SSE 文本（每行 `data:` 或物理行）。
- 网络层：对 `GET .../body` 使用 `**ReadableStream` 解码**，边下边喂给编辑器/缓冲，避免长期悬空的单次 `await response.text()`（视实现选型而定）。

### 5.3 文案

- 流式正文区域标明：**「原始 SSE（与客户端一致）」**；若 `metadata.stream_aborted`，显示 **「连接已中断，文件为已接收部分」**（不是截断，是事实陈述）。

---

## 6. 风险与运维


| 风险    | 缓解                                    |
| ----- | ------------------------------------- |
| 磁盘打满  | 与现有 `retention_days`、监控盘量一致；流式更易积压大文件 |
| 单文件极大 | 告警；必要时仅运维层限速/配额，产品仍不「截断内容」            |
| 浏览器卡顿 | 必选虚拟化/编辑器，避免纯 `<pre>` 全量挂载            |
| 权限    | body 与 download 与现有一致：仅归属用户可读         |


---

## 7. 验收标准（建议）

1. `stream: true` 且上游正常结束：磁盘存在完整 SSE 文件；审计行 **有** `response_body_path`；详情可加载 **全文**（无产品层长度截断）。
2. 客户端中途断开：文件为已收字节序列；审计行可区分 **完成 / 中止**。
3. 列表与导出：行内 token/finish（若实现）与流结束后一致或与产品声明的「最终一致性延迟」一致。
4. E2E：补充流式用例（可依赖 mock 上游返回 SSE）。

---

## 8. 分期建议


| 阶段      | 内容                                                             |
| ------- | -------------------------------------------------------------- |
| **MVP** | 流式 append 写盘 + UPDATE 审计行 + 去掉 body API 硬性字节上限 + 详情虚拟化/编辑器选型之一 |
| **M1**  | chunked/download 与详情大文件加载路径统一；断开连接 finalize；E2E                |
| **M2**  | 详情自动刷新「流结束后出现正文」；SSE 结构化折叠视图（可选）                               |


---

## 9. 文档与代码索引（实现时）

- 代理：`src/handlers/proxy.rs`（`is_stream` 分支）
- 落盘：`src/audit.rs`（`save_body_to_file` 扩展或 `append_stream_body`）
- 数据库：`src/db.rs`（UPDATE 审计行）
- 读正文 API：`src/handlers/audit.rs`（`get_audit_log_body`）
- 前端详情：`frontend/src/features/logs/pages/log-detail-page.tsx`

---

**变更记录**


| 日期         | 版本  | 说明                                             |
| ---------- | --- | ---------------------------------------------- |
| 2026-04-07 | 1.0 | 初稿：流式落盘、两段式 DB、`MAX_AUDIT_BODY_BYTES` 与前端不截断展示 |


