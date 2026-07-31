# Harness Engineering 实战指南

> Harness engineering(挽具工程)是通过改造 AI agent 周围的**软件环境**——上下文、工具、记忆、反馈循环、沙箱、可观测性——来提升 agent 输出质量和可靠性的工程学科。核心不是"调 prompt",而是"造系统"。

## 1. 核心定义

**Agent = Model + Harness。**

模型负责推理生成 token;harness 是包裹在模型外面的一切工程脚手架。LLM 默认是**无状态**的——每次会话从零开始,不会记长期上下文、不会判断何时重试、不会验证自己输出是否正确。这些责任全部落在 harness 上。

> 比喻:一匹马本身很有力,但没有挽具就无法耕地。缰绳和挽具让你把它的力量导向有用的工作。LLM 同理。

## 2. 核心心智模型:Guides + Sensors

Harness 由两类互补的控制组成:

| 类型 | 时机 | 作用 | 例子 |
|------|------|------|------|
| **Guides(前馈控制)** | agent 行动**之前** | 提高"第一次就做对"的概率 | AGENTS.md、架构规则、LSP、bootstrap 脚本 |
| **Sensors(反馈控制)** | agent 行动**之后** | 让 agent 自我纠错 | linter、测试、code review agent、mutation testing |

**关键技巧**:让 sensor 的输出**面向 LLM 优化**。比如 linter 不只报错,还附带"该怎么改"的指令,agent 就能自动闭环修复。

### 执行成本两分法

- **计算型(deterministic)**:类型检查、静态 lint、结构分析——毫秒级、确定、可每次跑。
- **推理型(inferential)**:AI code review、LLM-as-judge——慢、贵、非确定,但能给丰富语义反馈。

> 原则:**先铺满便宜可靠的计算型控制,再战略性地加推理型控制。**

### 三类调控维度

1. **可维护性 harness** —— 代码质量(用现成工具最容易做)
2. **架构适应度 harness** —— 用 fitness function 守护架构特征
3. **行为 harness** —— 功能正确性(最不成熟,仍高度依赖人工测试)

## 3. 五大可落地杠杆

按"什么时候用什么"来选:

| 选择 | 适用场景 |
|------|---------|
| **AGENTS.md** | 不断重复传递轻量级信息时 |
| **Skills** | 需要可复用的命名知识/工作流时 |
| **MCP Servers** | 需要频繁带鉴权访问外部服务时 |
| **Subagents** | 委派边界清晰/可并行的任务时 |
| **Hooks** | 需要确定性机械逻辑时 |

### 3.1 AGENTS.md —— 最轻量的项目级引导

- **写**:常用命令、与默认不同的代码风格、测试方法、仓库约定、架构决策、环境怪癖、关键提示。
- **不写**:agent 能从代码推断出的东西、标准惯例、详细 API 文档、频繁变动的数据、长教程、逐文件描述。
- 保持简洁;内容变大时把扩展章节挪到 `/docs/`。

### 3.2 Skills —— 可复用的命名知识包

重复的 prompt 就该固化成 skill。第三方 skill 用前**先审计源码**安全风险。

### 3.3 MCP Servers —— 接外部带鉴权服务

适合 Linear、Figma、Slack、Sentry 等。**注意**:把所有 MCP 工具信息塞进 prompt 会稀释 agent 能力,可考虑用 CLI 包装。

### 3.4 Subagents —— 边界清晰的任务委派

核心收益是**隔离上下文**,避免主线被探索笔记和走错的尝试污染,结果更干净聚焦。

### 3.5 Hooks —— 确定性机械逻辑

自动化格式化、lint、审批门、通知、建 PR。设计原则:**成功时静默,只在需要人介入的失败时发声。**

## 4. 落地工作流

1. **盘点失败模式** —— 记录 agent 反复做错的事,这是你的需求清单。
2. **优先级排序** —— 先做"高影响 + 容易实现"的控制。
3. **迭代纠偏循环** —— 每当某个错误出现 ≥2 次,就工程化一个控制确保它**永不再犯**(这是 harness engineering 的精髓)。
4. **逐层加复杂度** —— 先计算型(便宜可靠),再按需加推理型。
5. **Keep Quality Left** —— 按成本/速度把控制分布到生命周期:
   - **提交前**:linter、快速测试、基础 review agent
   - **集成后**:mutation testing、完整 review
   - **持续监控**:漂移检测、运行时反馈(SLO、异常)
6. **为"可挽具化"而设计** —— 选择本身就便于监控和控制的技术栈与架构。

## 5. 最小起步清单

- [ ] 在仓库根写一个精简的 **AGENTS.md**(命令 + 测试 + 风格差异 + 架构红线)
- [ ] 配好 **pre-commit 的 linter / 类型检查**,并让报错信息带修复指引
- [ ] 把最常重复的 prompt 固化成 **1 个 skill**
- [ ] 加一个 **hook**:自动格式化 / 失败时通知
- [ ] 建立"**错误日志**",每个重复错误 → 加一条对应的 guide 或 sensor

## 6. 关于人的角色

> "好的 harness 不应该追求完全消除人的输入,而是把人的输入导向最重要的地方。"

Harness 把工程师隐性的"什么是好代码"显式化、自动化,但无法替代组织对齐和情境判断。人类带来的隐性问责与直觉是 agent 缺乏的——harness 的工作是把工程师对"好"的隐性认知变成显式规则。

## 参考资料

- [Harness engineering for coding agent users — Martin Fowler](https://martinfowler.com/articles/harness-engineering.html)
- [Harness engineering — Software Mansion Agentic Engineering Guide](https://agentic-engineering.swmansion.com/becoming-productive/harness-engineering/)
- [What Is an Agent Harness? — Firecrawl](https://www.firecrawl.dev/blog/what-is-an-agent-harness)
- [Agent Harness Engineering — Addy Osmani](https://addyosmani.com/blog/agent-harness-engineering/)
- [awesome-harness-engineering — GitHub](https://github.com/ai-boost/awesome-harness-engineering)
