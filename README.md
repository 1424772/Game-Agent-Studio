# Game Agent Studio

一个本地优先的多 Agent 游戏创作工作台，帮助独立游戏开发者从零到一创作游戏。

## 产品定位

这是一个本地 AI 游戏创作操作系统（AI Game Creation OS），不是简单的聊天软件。支持多 Agent 协作、本地项目管理、自迭代工作流、Agentic RAG 知识检索，以及导出标准化游戏设计文档和结构化数据。

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
| RAG 检索 | SQLite LIKE + 向量混合检索（Hybrid RAG） |
| OCR | Tesseract OCR (MCP Server, 本地离线) |
| i18n | React Context + localStorage（中文 / English） |
| 数据格式 | SQLite + JSON + Markdown |

## 安全架构

### API Key 保护
- 前端 **永不持有完整 API Key**。`get_model_config` 只返回 `has_api_key` 和脱敏后的 `masked_api_key`
- `run_llm_completion` 由 Rust 侧从加密数据库读取并解密 API Key，前端 `LlmRequest` 不含 `api_key` 字段
- 存储：`LocalEncryptedSecretStore`（AES-256-GCM），`KeychainSecretStore` 接口预留（OS Keychain）
- 错误脱敏：`sanitize_error()` 覆盖 Bearer token、Authorization header、`sk-` 前缀、`api_key=` 参数、请求体中的密钥

### 权限边界
- 零插件权限：`capabilities/default.json` 的 `permissions: []`，无 shell/fs/dialog/clipboard/notification
- CSP prod: `default-src 'self'; connect-src 'self' ipc:`
- CSP dev: 额外允许 `http://localhost:1420 ws://localhost:1421`（Vite HMR）

### 网络边界
- `validate_base_url()` 使用 `url::Url` parser，生产环境仅允许 `https://` public host
- 拒绝 `file://`、`ftp://`、`data:`、`javascript:`、空 host
- 生产拒绝 localhost / loopback / private / link-local / unspecified IP
- Debug 仅允许 exact localhost / 127.0.0.1 / ::1 的 http
- `reqwest` 60s timeout，消息 ≤20 条、≤40000 chars、max_tokens ≤32768

### 文件写入边界
- 导出路径固定在 `{app_data}/game-agent-studio/exports/`
- 路径 `canonicalize` + 白名单校验，文件名 `sanitize_project_name()` 清洗
- 导出内容排除 `qa_review`、`system_internal`，不含模型配置

### 自迭代安全
- 提案生成器只分析事件数据，**不自动修改源码/配置/文件**
- Code/Prompt/Safety/Export/DataModel 类 proposal 默认 `requires_human_approval=true`
- Accept/Reject 仅更新 proposal 状态 + 审计事件，不触发任何自动操作
- 状态流转校验：draft→proposed→accepted/rejected→implemented/superseded

### RAG 安全
- 所有检索结果写入前经 `sanitize_error` 脱敏
- Agent prompt 注入的 chunk excerpt 同样脱敏
- UI 搜索结果 excerpt 脱敏后展示

## 核心功能

### 1. 项目管理
- 新建/打开/删除项目，支持 card_game / visual_novel / rpg 等游戏类型
- 项目列表卡片展示

### 2. 模型设置
- 配置 OpenAI-compatible API（Base URL、API Key、Model、Temperature、Max Tokens）
- URL 校验（https-only 生产策略）、max_tokens 上限 32768
- API Key 加密存储，前端仅显示脱敏形式
- 保存的 Key 空输入时测试连接自动回退到已存储 Key

### 3. Agent 工作台
- `run_workflow` 全流程在 Rust 侧编排（ProducerAgent → GameDesignerAgent → QAAgent）
- Workflow Registry 定义 3 套 workflow × 3 steps，每 step 带 step_key + use_rag 标志
- Agent Steps upsert by `(run_id, step_key)`，None 不覆盖已有数据
- Run status: running → completed/failed，失败自动记录错误
- 消息状态校验（非法状态返回 Err），编辑内容写入 `message_revisions` 表
- 每个 step 执行完毕自动写入 `project_memory`（L2，含 provenance）

### 4. Agentic RAG
- 文档导入 → 段落分块（~2000 chars）→ chunk metadata（含 source/provenance/content_hash）
- 关键词检索（LIKE + 词频打分 + 排序），limit clamp 1-20，最多 20 个搜索词
- `retrieve_for_context` 内部 service 被 UI 搜索和 Agent workflow 共享
- Designer/QA step 设置了 `use_rag: true`，执行时自动检索知识库并注入 prompt
- `retrieval_hits.used_by_agent` 记录 run_id/step_key/agent_name 追踪
- `agent_steps.input_json` 含 `retrieval_run_id` + `retrieval_hits[]` 结构化审计数据
- 检索结果全部在同一事务中写入 `retrieval_runs` + `retrieval_hits`

### 5. 事件日志
- 17 个固定事件类型常量（workflow_start/complete/failed、step_start/complete/failed、output_accepted/rejected/edited、proposal_created/reviewed、export_created、memory_saved 等）
- 字段：`run_id`、`actor`、`severity`、`correlation_id`、`redaction_level`
- 支持按 project_id / run_id / correlation_id / event_type 过滤查询
- 所有 event_data 写入前经 `sanitize_error`

### 6. 四层记忆
- L1 Session / L2 Project / L3 User Preference / L4 System Evolution
- `save_project_memory` 校验 memory_type allowlist (12 种)、layer/scope 组合合法性、confidence 0-1、version ≥1
- 更新记忆时自动写入 `memory_versions` 表（old_value/new_value/provenance），旧值永不丢失
- 事务原子化：memory_versions INSERT + project_memory UPDATE
- 返回原始 created_at

### 7. 导出中心
- Markdown / JSON 导出，路径安全约束
- `export_created` 审计事件，含 correlation_id
- 导出历史可追溯

### 8. 自迭代面板
- 从真实事件数据生成 proposal（低接受率 → prompt_improvement，高编辑率 → prompt_improvement，多次失败 → workflow_improvement）
- Accept/Reject 更新 proposal 状态 + 写入 `proposal_reviewed` 审计事件
- Pending 列表只显示 `proposed` 状态，不含 draft

### 9. 知识库
- 文档创建/列表/分块/搜索/检索历史
- 搜索结果展示 doc_title / doc_type / excerpt / score / rank / source / provenance
- Retrieval history 显示查询词/命中数/耗时

## 目录结构

```
BuildGameAgent/
├── index.html
├── package.json
├── vite.config.ts
├── tsconfig.json
├── .cargo/config.toml
│
├── src/                            # 前端
│   ├── main.tsx / App.tsx
│   ├── app/
│   │   ├── components/             # Layout, Sidebar
│   │   ├── routes/
│   │   │   ├── ProjectDashboard.tsx
│   │   │   ├── ModelSettings.tsx
│   │   │   ├── AgentWorkspace.tsx
│   │   │   ├── MemoryCenter.tsx
│   │   │   ├── KnowledgeBase.tsx
│   │   │   ├── ExportCenter.tsx
│   │   │   └── SelfIterationPanel.tsx
│   │   ├── stores/                 # useAppStore, useProjectStore, useModelStore, useAgentStore
│   │   └── styles/global.css
│   └── shared/
│       ├── types/index.ts
│       ├── constants/index.ts
│       └── utils/tauri.ts
│
├── src-tauri/                      # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/default.json   # permissions: []
│   └── src/
│       ├── main.rs / lib.rs
│       ├── models/mod.rs           # 数据模型 + sanitize_error
│       ├── crypto/mod.rs           # SecretStore trait + LocalEncryptedSecretStore
│       ├── db/                     # init, migrations (15 tables)
│       └── commands/
│           ├── projects.rs         # 项目 CRUD
│           ├── model_configs.rs    # 模型配置 + URL 校验
│           ├── agents.rs           # Agent 编排 + run_workflow + RAG 集成
│           ├── workflow.rs         # Workflow registry + Agent registry
│           ├── events.rs           # 事件日志 + 查询过滤器
│           ├── memory.rs           # 四层记忆 + memory_versions
│           ├── exports.rs          # Markdown/JSON 导出
│           ├── iterations.rs       # 改进提案 CRUD + 状态流转
│           ├── rag.rs              # 文档/分块/检索 + retrieve_for_context
│           └── security.rs         # URL 校验 + LLM 请求限额 + command manifest
```

## 数据库表

| 表名 | 用途 | 状态 |
|------|------|------|
| `projects` | 项目基本信息 | 已实现 |
| `model_configs` | LLM 配置（AES-256-GCM 加密） | 已实现 |
| `agent_runs` | Agent 执行记录 | 已实现 |
| `agent_steps` | Agent 每步输入/输出/token（UNIQUE run_id+step_key） | 已实现 |
| `agent_messages` | Agent 消息（含用户接受/拒绝/编辑状态） | 已实现 |
| `message_revisions` | 消息编辑历史 | 已实现 |
| `events` | 增强事件日志（17 种事件类型） | 已实现 |
| `project_memory` | 四层记忆（layer/scope/confidence/version/provenance） | 已实现 |
| `memory_versions` | 记忆历史版本（old_value/new_value） | 已实现 |
| `user_preferences` | 用户偏好（含置信度和证据） | 已实现 |
| `exports` | 导出记录 | 已实现 |
| `documents` | RAG 文档 | 已实现 |
| `document_chunks` | RAG 文档分块（含 metadata） | 已实现 |
| `retrieval_runs` | RAG 检索记录 | 已实现 |
| `retrieval_hits` | RAG 检索命中（含 used_by_agent） | 已实现 |
| `improvement_proposals` | 系统改进建议（含 target_area/proposed_change） | 已实现 |

## 命令清单（40 个）

```
chunk_document                get_memory_versions           run_llm_completion
create_agent_run              get_model_config              run_workflow
create_document               get_project                   save_agent_message
create_improvement_proposal   get_project_memory            save_agent_step
create_project                get_retrieval_hit_excerpts    save_model_config
delete_project                get_retrieval_hits            save_project_memory
export_json                   get_retrieval_runs            search_documents
export_markdown               get_user_preferences          test_model_connection
get_agent_messages            list_documents                update_agent_message_content
get_agent_run                 list_improvement_proposals    update_agent_run
get_agent_runs                list_projects                 update_message_status
get_agent_steps               log_event                     update_user_preferences
get_document_chunks           review_improvement_proposal
get_events
get_exports
```

HANDLER_NAMES / ALLOWED_COMMANDS 双重清单 + `generate_handler![]` 源码解析测试确保一致。

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
| `ProposalStatus` | draft → proposed → accepted/rejected → implemented/superseded |

## 环境要求

- **Node.js** >= 18
- **Rust** >= 1.70（`x86_64-pc-windows-gnu` 工具链）
- **MinGW-w64**（推荐 WinLibs 或 LLVM-MinGW）
- **Windows 10/11**

## 快速开始

```bash
npm install
npm run tauri dev       # 开发模式
npm run tauri build     # 生产构建
cd src-tauri && cargo check  # Rust 快速检查
```

## 架构变更记录

### v0.2.0 — P1~P6 全栈交付
- **P1 安全阻断**: API Key 加密 + 脱敏返回 + 导出路径安全约束 + 错误全路径 sanitize
- **P2 权限与网络**: 零插件权限 + CSP dev/prod 拆分 + URL parser 校验 + reqwest timeout + 请求限额
- **P3 Agent 编排**: Workflow registry + run_workflow Rust 编排 + step upsert + 消息 revision + 审计事件链
- **P4 记忆与事件**: 四层记忆语义 + memory_versions + layer/scope 校验 + 17 种事件类型 + get_events 过滤器
- **P5 自迭代**: Proposal CRUD + 状态流转 + requires_human_approval + proposal_created/reviewed 审计
- **P6 Agentic RAG**: Document/chunk/retrieval + retrieve_for_context + Agent workflow 接入 RAG + used_by_agent 追踪 + excerpt 脱敏
- **i18n**: 中英文双语界面，侧边栏语言切换，localStorage 持久化
- **Logo**: 自定义应用图标
- **V1 硬化**: 事务原子化（proposal/event 同事务）、安全回归测试 15+、发布检查清单 + 安全审查文档
- **P7 OS Keychain**: 系统级密钥存储（Windows Credential Manager / macOS Keychain / Linux Secret Service），自动迁移旧加密密钥
- **P8 Embedding + Hybrid RAG**: 向量检索（cosine similarity）、hybrid keyword+vector 合并去重、keyword/vector/hybrid/keyword_fallback 四策略、embed_pending_chunks + per-chunk 校验隔离、score_breakdown 审计
- **P9a Token 预算 + RAG 压缩**: context_token_budgeting（truncate_str + truncate_rag_context）、Jaccard deduplication、injected/truncated/deduped_out 状态追踪、字符级预算精确控制

### 迁移风险
- **API Key 需重新配置**：旧数据 `api_key` 列已迁移为 `encrypted_api_key`
- **导出路径变更**：不接受自定义 `output_dir`，统一写入应用数据目录
- **Keychain 迁移**：首次启动自动迁移旧密文到 OS keychain，失败保留旧存储

## OCR 配置（用于 DeepSeek 等无多模态模型）

本机已配置 Tesseract OCR MCP Server，OpenCode 可通过 OCR 工具读取图片中的文字。

### 组件清单

| 组件 | 位置 |
|---|---|
| Tesseract OCR 引擎 | `C:\Program Files\Tesseract-OCR\tesseract.exe` |
| 语言包 (eng + chi_sim) | `D:\tessdata\` |
| MCP 服务脚本 | `tools/ocr_server.py` |
| opencode 配置 | `opencode.json` |

### 使用

重启 OpenCode 后，在对话中拖入图片或指定路径，调用 `ocr` 工具即可提取文字。无需 API Key，纯本地离线运行。

## 后续计划

- [ ] 流式 LLM 响应
- [x] 向量检索 + Embedding 集成 (P8)
- [x] 上下文 Token 预算 + RAG 去重 (P9a)
- [ ] 更多 Agent 类型（CardGameAgent、VNAgent 等）
- [x] OS Keychain 集成 (P7)
- [ ] Web 小游戏 / 微信小游戏导出
- [ ] Godot / Ren'Py / Phaser 项目模板导出
- [ ] 工作流可视化编辑器

## 许可

仅限个人使用，不开放分发许可。
