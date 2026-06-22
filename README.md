# Game Agent Studio

一个本地优先的多 Agent 游戏创作工作台，帮助独立游戏开发者从零到一创作游戏。

## 产品定位

这是一个本地 AI 游戏创作操作系统（AI Game Creation OS），不是简单的聊天软件。支持多 Agent 协作、本地项目管理、自迭代工作流，以及导出标准化游戏设计文档和结构化数据。

## 技术栈

| 层次 | 技术 |
|------|------|
| 桌面应用 | Tauri v2 |
| 前端 | React 18 + TypeScript + Vite 5 |
| 状态管理 | Zustand |
| 后端 | Rust |
| 数据库 | SQLite（本地，WAL 模式） |
| 加密 | AES-256-GCM（API Key 加密存储） |
| LLM 调用 | reqwest（Rust 侧，前端不接触 API Key） |
| LLM Provider | OpenAI-compatible API（用户可自定义 base_url / api_key / model） |
| 数据格式 | SQLite + JSON + Markdown |

## 安全架构

### API Key 保护
- 前端 **永不持有完整 API Key**。`get_model_config` 只返回 `has_api_key` 和脱敏后的 `masked_api_key`
- `run_llm_completion` 由 Rust 侧从加密数据库读取并解密 API Key，前端 `LlmRequest` 不含 `api_key` 字段
- 存储：AES-256-GCM 加密，密钥由本机 hostname + 盐值派生（V2 将升级为 OS Keychain）
- 所有错误返回经 `sanitize_error()` 脱敏，清除 Bearer token、`sk-` 前缀密钥、请求体中的 api_key

### 权限边界
- 已移除 `tauri-plugin-shell`，无系统 Shell 调用权限
- CSP: `default-src 'self'; connect-src 'self' ipc: http://localhost:1420`
- Capabilities: 仅 `core:default`，只允许主窗口调用已注册的 Tauri Command

### 文件写入边界
- 导出路径固定在 `{app_data}/game-agent-studio/exports/`，不接受用户自定义路径
- 路径经 `canonicalize` + 白名单校验，防止目录穿越
- 项目名经 `sanitize_project_name()` 清洗，移除路径分隔符、控制字符、Windows 保留名
- 导出内容排除 `qa_review`、`system_internal` 类型记忆，不含模型配置或系统内部信息

### 自迭代安全
- 第一版只生成改进建议，**不自动修改源码**
- 建议存储在 `improvement_proposals` 表，标记 `requires_human_approval: true`

## 核心功能（MVP v0.1.0）

### 1. 项目管理
- 新建项目（支持卡牌游戏、视觉小说类型）
- 打开/删除已有项目
- 项目列表卡片展示

### 2. 模型设置
- 配置 OpenAI-compatible API（Base URL、API Key、Model、Temperature、Max Tokens）
- 一键测试模型连接
- API Key 加密存储，前端仅显示脱敏形式（`sk-...xxxx`）

### 3. Agent 工作台
- ProducerAgent → GameDesignerAgent → QAAgent 顺序执行
- 每个 Agent Step 的输入、输出、token 用量、状态、错误全部持久化到 `agent_steps` + `agent_messages`
- Agent 输出自动保存到项目记忆（L2）
- 用户可接受/拒绝/编辑 Agent 输出，触发 `update_message_status` 写入数据库
- 支持 `card_game_concept` 和 `visual_novel_concept` 工作流

### 4. 事件日志
- 全量记录项目创建、Agent 运行、输出接受/拒绝/编辑、导出等事件
- 字段：`run_id`、`actor`、`severity`、`correlation_id`、`redaction_level`
- 为后续用户偏好分析和工作流优化提供审计追踪

### 5. 记忆中心
- 按记忆类型（世界观、角色、剧情、规则、美术风格等）分组查看
- 每条记忆含：`layer`(L1-L4)、`scope`、`source`、`confidence`(0-1)、`version`、`provenance`
- 支持按记忆类型筛选

### 6. 导出中心
- 导出 Markdown 格式游戏设计文档（按记忆类型分章节）
- 导出 JSON 格式结构化项目数据
- 导出路径受安全边界约束，导出历史可追溯

### 7. 自迭代面板
- 展示系统事件记录
- 基于事件模式生成改进洞察

## 目录结构

```
BuildGameAgent/
├── index.html                      # 入口 HTML
├── package.json                    # 前端依赖配置
├── vite.config.ts                  # Vite 配置
├── tsconfig.json                   # TypeScript 配置
├── .cargo/config.toml              # Rust 编译配置（GNU 链接器标志）
│
├── src/                            # 前端源码
│   ├── main.tsx                    # React 入口
│   ├── App.tsx                     # 主应用（路由分发）
│   ├── app/
│   │   ├── components/
│   │   │   ├── Layout.tsx          # 主布局（侧边栏+内容区）
│   │   │   └── Sidebar.tsx         # 侧边导航栏（7 个页面入口）
│   │   ├── routes/
│   │   │   ├── ProjectDashboard.tsx    # 项目列表页
│   │   │   ├── ModelSettings.tsx       # 模型配置页
│   │   │   ├── AgentWorkspace.tsx      # Agent 工作台
│   │   │   ├── MemoryCenter.tsx        # 记忆中心
│   │   │   ├── ExportCenter.tsx        # 导出中心
│   │   │   └── SelfIterationPanel.tsx  # 自迭代面板
│   │   ├── stores/
│   │   │   ├── useAppStore.ts      # 全局应用状态（路由、当前项目）
│   │   │   ├── useProjectStore.ts  # 项目 CRUD + 事件日志
│   │   │   ├── useModelStore.ts    # 模型配置（不持有完整 API Key）
│   │   │   └── useAgentStore.ts    # Agent 编排（3-step pipeline + 记忆写入）
│   │   └── styles/
│   │       └── global.css          # 全局样式（暗色主题）
│   ├── features/                   # V1 逻辑在 stores/commands 中，目录预留给后续解耦
│   │   ├── agents/.gitkeep
│   │   ├── workflows/.gitkeep
│   │   ├── memory/.gitkeep
│   │   ├── rag/.gitkeep
│   │   ├── exports/.gitkeep
│   │   └── self_iteration/.gitkeep
│   └── shared/
│       ├── types/index.ts          # TypeScript 类型定义（含 union types）
│       ├── constants/index.ts      # 常量定义
│       └── utils/tauri.ts          # Tauri invoke 封装（类型安全）
│
├── src-tauri/                      # Rust 后端源码
│   ├── Cargo.toml                  # Rust 依赖配置
│   ├── tauri.conf.json             # Tauri 应用配置（严格 CSP）
│   ├── build.rs                    # Tauri 构建脚本
│   ├── capabilities/
│   │   └── default.json            # 权限白名单（core:default only）
│   ├── icons/                      # 应用图标
│   └── src/
│       ├── main.rs                 # Rust 入口
│       ├── lib.rs                  # Tauri 应用初始化 + 命令注册
│       ├── models/mod.rs           # 数据模型、枚举、脱敏函数
│       ├── crypto/mod.rs           # SecretStore（AES-256-GCM 加解密）
│       ├── db/
│       │   ├── mod.rs
│       │   ├── init.rs             # 数据库初始化（WAL 模式）
│       │   └── migrations.rs       # 数据表创建 + ALTER TABLE + 索引
│       └── commands/
│           ├── mod.rs
│           ├── projects.rs         # 项目 CRUD
│           ├── model_configs.rs    # 模型配置（加密存储 + 脱敏返回）
│           ├── agents.rs           # Agent 运行 + Agent Steps + 消息状态 + LLM 调用
│           ├── events.rs           # 事件日志（含增强字段）
│           ├── memory.rs           # 项目记忆 + 用户偏好
│           └── exports.rs          # Markdown/JSON 导出（路径安全约束）
│
└── data/                           # 数据模板目录
    ├── templates/
    │   ├── card_game/              # 卡牌游戏模板
    │   ├── visual_novel/           # 视觉小说模板
    │   └── exports/                # 导出模板
    └── knowledge/
        ├── game_design/            # 游戏设计知识库
        ├── godot/                  # Godot 导出知识
        ├── renpy/                  # Ren'Py 导出知识
        └── phaser/                 # Phaser 导出知识
```

## 数据库表

| 表名 | 用途 | V1 状态 |
|------|------|---------|
| `projects` | 项目基本信息 | 已实现 |
| `model_configs` | LLM 模型配置（API Key AES-256-GCM 加密） | 已实现 |
| `agent_runs` | Agent 执行记录 | 已实现 |
| `agent_steps` | Agent 每一步的输入/输出/token/状态 | 已实现 |
| `agent_messages` | Agent 消息（含用户接受/拒绝/编辑状态） | 已实现 |
| `events` | 增强事件日志（run_id, actor, severity 等） | 已实现 |
| `project_memory` | 四层记忆（layer/scope/confidence/version/provenance） | 已实现 |
| `user_preferences` | 用户偏好（含置信度和证据） | 已实现 |
| `exports` | 导出记录 | 已实现 |
| `documents` | RAG 文档（预留） | 表已创建 |
| `document_chunks` | RAG 文档分块（预留） | 表已创建 |
| `retrieval_runs` | RAG 检索记录（预留） | 表已创建 |
| `retrieval_hits` | RAG 检索命中（预留） | 表已创建 |
| `improvement_proposals` | 系统改进建议（预留） | 表已创建 |

## 数据模型枚举

| 枚举 | 值 |
|------|-----|
| `GameType` | card_game, visual_novel, rpg, puzzle, strategy, simulation |
| `WorkflowType` | card_game_concept, visual_novel_concept, game_design_doc |
| `AgentRunStatus` | pending, running, waiting_for_input, completed, failed, cancelled |
| `MessageStatus` | pending, streaming, completed, failed, cancelled, accepted, rejected, edited |
| `MemoryLayer` | L1, L2, L3, L4 |
| `MemoryScope` | project, session, global |
| `EventSeverity` | debug, info, warning, error, critical |
| `ProposalType` | workflow_improvement, prompt_improvement, code_improvement, export_template_fix, safety_enhancement, ui_ux_improvement, data_model_refinement |
| `ProposalStatus` | draft, proposed, accepted, rejected, implemented, superseded |

## 四层记忆结构

| 层级 | 名称 | 存储位置 | 内容 |
|------|------|---------|------|
| L1 | 会话记忆 | `agent_messages` / 内存 | 当前任务上下文、对话目标、临时约束、Agent 执行状态 |
| L2 | 项目记忆 | `project_memory` (layer=L2) | 世界观、角色、剧情、规则、卡牌/道具/关卡、美术风格、已否定方案 |
| L3 | 用户偏好记忆 | `user_preferences` | 偏好游戏类型、平台、美术风格、常用模型、文案长度偏好等 |
| L4 | 系统进化记忆 | `events` / `improvement_proposals` | Agent 工作流成功率、Prompt 模板效果、用户接受/拒绝的系统改进 |

## Loop Engineering 核心循环

```
Observe → Retrieve → Plan → Generate → Critique → Revise → Test → Commit → Learn
```

当前 V1 实现：Plan(ProducerAgent) → Generate(GameDesignerAgent) → Critique(QAAgent)，Commit 通过 `saveDesignToMemory` 写入 L2 记忆，Learn 通过 `logEvent` 写入审计日志。

## 多 Agent 设计

### 通用 Agent
- **ProducerAgent**：拆解任务、选择工作流、协调其他 Agent
- **GameDesignerAgent**：玩法设计、核心循环设计
- **NarrativeAgent**：世界观、剧情、角色、分支
- **RuleAgent**：规则系统、数值结构
- **ArtDirectorAgent**：美术风格、视觉规范
- **QAAgent**：检查漏洞、冲突、不一致、范围失控
- **ExportAgent**：导出 Markdown / JSON / CSV / 模板项目
- **MemoryAgent**：提炼用户偏好、项目记忆
- **SelfIterationAgent**：提出系统改进建议

### 类型 Agent
- **CardGameAgent**：卡牌机制、卡池、费用、关键词、战斗规则
- **CardBalanceAgent**：卡牌强度、费用曲线、组合风险
- **VNAgent**：视觉小说章节、分支、对话、角色关系
- **PuzzleAgent**（后续扩展）
- **RPGAgent**（后续扩展）
- **WeChatMiniGameAgent**（后续扩展）

## 环境要求

- **Node.js** >= 18
- **Rust** >= 1.70（需 `x86_64-pc-windows-gnu` 工具链）
- **MinGW-w64**（GNU 工具链的链接器，推荐 WinLibs 或 LLVM-MinGW）
- **Windows 10/11**

## 快速开始

### 安装依赖

```bash
# 前端依赖
npm install

# Rust 工具链（如未安装）
rustup default stable-x86_64-pc-windows-gnu
```

### 开发模式

```bash
npm run tauri dev
```

### 生产构建

```bash
npm run tauri build
```

### 只构建前端

```bash
npm run build
```

### 只检查 Rust 后端

```bash
cd src-tauri
cargo check       # 快速检查（推荐开发时使用）
cargo build       # 完整编译
```

## 使用流程

1. 启动应用，进入 Project Dashboard
2. 点击「新建项目」，选择游戏类型，填写名称和描述
3. 进入 Model Settings，配置 OpenAI-compatible API（Base URL、API Key、模型）
4. 点击「测试连接」确认配置正确
5. 返回 Project Dashboard，打开刚创建的项目
6. 在 Agent Workspace 中选择工作流类型，输入任务描述
7. 点击「运行工作流」，等待 Agent 顺序执行
8. 每个 Agent Step 的输出在界面上展示，可接受、拒绝或编辑
9. 在 Memory Center 查看保存的项目记忆
10. 在 Export Center 导出 Markdown 或 JSON（导出到应用数据目录）

## 架构变更记录

### v0.1.1 — 安全加固
- API Key: 移除前端持有，Rust 侧加密存储 + 脱敏返回
- 移除 `tauri-plugin-shell`，收紧权限边界
- CSP 从 `null` 改为严格策略
- 导出路径锁定到应用数据子目录，文件名清洗
- 事件日志、项目记忆增强字段（layer/scope/confidence/version/severity 等）
- 新增 `agent_steps` 表，持久化每步输入输出和 token 用量
- `update_message_status` 真实写入数据库
- 预留 RAG 和自迭代表结构

### 迁移风险
- **API Key 需重新配置**：`model_configs` 表列名从 `api_key` 改为 `encrypted_api_key`，旧数据不兼容
- **导出路径变更**：不再接受自定义 `output_dir`，统一写入 `{app_data}/game-agent-studio/exports/`

## 后续计划

- [ ] 流式 LLM 响应
- [ ] Agentic RAG（向量检索 + Hybrid Search）
- [ ] 更多 Agent 类型（CardGameAgent、VNAgent 等）
- [ ] OS Keychain 集成（替换 hostname 派生密钥）
- [ ] Web 小游戏导出
- [ ] 微信小游戏导出
- [ ] Godot / Ren'Py / Phaser 项目模板导出
- [ ] 工作流可视化编辑器
- [ ] 游戏原型预览

## 许可

仅限个人使用，不开放分发许可。
