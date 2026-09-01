# AGENTS.md

## Applicable scope

This specification applies to all directories and files in this repository.
If subdirectories contain their own AGENTS.md, sub-level rules may supplement but must never weaken the constraints in this file.

## Project overview

Tauri v2 desktop app with React 19 + TypeScript frontend (Vite) and Rust backend. Early stage (v0.1.0), scaffolded from the official Tauri template. Project name: Mnemosyne (Greek titaness of memory, mother of the nine Muses).

## Project goals & coding principles

- The project is a greenfield system. **Consistency** and **simplicity** are the highest priorities.
- Never introduce redundant branches, compatibility layers, dual-track logic, or patches "for backward compatibility."
- New features and refactors must serve consistency, maintainability, and readability — not historical baggage.
- **No `any` types.** All TypeScript must have explicit types.
- **No hardcoded language strings.** Use i18n per project conventions. ## Tauri security model

### Trust boundary

| Trust zone | Capability | Constraint |
|-----------|-----------|-----------|
| **Rust core** | Full OS access | Unrestricted |
| **WebView frontend** | IPC calls only | Only declared commands callable |

Core rules:
- Frontend code is never trusted — always validate all input received from frontend.
- Sensitive logic and business data must live in Rust core, never in frontend.
- IPC is the sole communication bridge; all cross-boundary data must be validated.

### Capabilities system

Capabilities are defined in `src-tauri/capabilities/*.json`. Only declared commands are callable from frontend.

- Default: `core:default`, `opener:default`
- Add new Tauri plugin permissions in `src-tauri/capabilities/default.json`
- Remote sources must be explicitly declared to access Tauri commands
- Permission granularity: prefer operation-level (e.g. `window:allow-set-title`) over broad resource-level grants

### Input validation

- All `#[command]` functions must validate input (type, length, format).
- Path operations must sanitize `../` traversal.
- Never expose internal details (full paths, stack traces) to frontend — map to user-friendly messages.
- Secrets and tokens must never pass through `invoke`.
- **CSP**: Currently disabled (`null`). Must be re-enabled before production with strict policy (no `unsafe-eval`).

## Security Kernel 模型

Security Kernel 是 Rust 核心的统一安全入口，所有敏感操作必须经过 Kernel 审批。

### 架构图（统一入口）

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SecurityKernel.execute()                           │
│                              （唯一安全入口）                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ Validation  │→ │   Policy    │→ │   Rate      │→ │  Resource   │        │
│  │   Layer     │  │   Engine    │  │  Limiter    │  │  Manager    │        │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘        │
│         │               │               │               │                   │
│         ↓               ↓               ↓               ↓                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Path      │  │  Approval   │  │   Rate      │  │   Quota     │        │
│  │  Sanitizer  │  │   Manager   │  │   Store     │  │   Check     │        │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘        │
│         │               │               │               │                   │
│         ↓               ↓               ↓               ↓                   │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │                     Permission Manager                           │       │
│  │  (Session-based, Capability-check, Scope-validation)            │       │
│  └─────────────────────────────────────────────────────────────────┘       │
│                                    │                                        │
│                                    ↓                                        │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │                        Audit Event Bus                           │       │
│  │  (LoggingHandler, MetricsHandler, SecurityEvent stream)         │       │
│  └─────────────────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ↓
                          ┌─────────────────┐
                          │    Executor     │
                          │  (Actual Op)    │
                          └─────────────────┘
```

### 执行流程

```rust
SecurityKernel::execute(op, ctx, executor)
  → ValidationLayer::validate_operation(op)         // 输入校验（路径/URL/ID）
  → PolicyEngine::evaluate(op, ctx)                 // Policy 决策
      → if Deny: emit PolicyDenied, return Err
      → if RequireApproval: 
          → if ctx.approval_token present:
              → ApprovalManager::validate(token, op, workspace)
          → else: emit ApprovalRequested, return Err
      → if Allow: continue
  → RateLimiter::check_and_fail(op, workspace)      // 频率限制
  → ResourceManager::check_quota(workspace)         // 资源配额
  → PermissionManager::check(op, workspace)         // Capability 校验
  → executor()                                       // 执行实际操作
  → AuditEventBus::emit(OperationComplete)          // 审计日志
```

### Rust Unknown Agent 原则

**核心原则**：前端 Agent 永远不被信任，所有操作请求必须经过 Security Kernel 审批。

| Agent 来源 | 默认信任级别 | Policy 行为 |
|:-----------|:-------------|:------------|
| **Unknown Agent**（前端未注册） | `Dangerous` | 默认 Deny 所有操作，必须显式批准 |
| **Registered Agent**（有 manifest） | 根据插件风险级别 | 按 Global Policy + Plugin Policy 评估 |
| **Main Agent**（内置核心） | `Trusted` | 允许大部分操作，高风险需 Approval |
| **Sub-Agent**（由 Main Agent 创建） | 继承父 Agent 信任级别 | 受父 Agent Policy 约束 |

**设计动机**：
- 前端 WebView 是不可信边界，任何来自前端的"Agent"都可能是被篡改的
- 插件系统允许第三方扩展，必须通过 Plugin Registry 注册并声明权限
- Unknown Agent 默认 Dangerous 防止零信任漏洞

### Permission Session 设计（无 Token）

Permission Manager 使用 **Session-based** 权限模型，而非传统 Token-based：

```
┌─────────────────────────────────────────────────────────────┐
│                    PermissionSession                        │
├─────────────────────────────────────────────────────────────┤
│  session_id: SessionId                                      │
│  workspace: WorkspaceId                                     │
│  capabilities: Vec<CapabilityId>    // 已授予的能力         │
│  expires_at: DateTime<Utc>           // Session TTL         │
│  created_by: UserId                  // 创建者              │
├─────────────────────────────────────────────────────────────┤
│  ✅ 无 approval_token 存储                                  │
│  ✅ 无 bearer_token 传递                                    │
│  ✅ 权限由 Session + Capability 组合决定                    │
│  ✅ Session 短生命周期（默认 30 分钟）                       │
└─────────────────────────────────────────────────────────────┘
```

**设计原则**：
- **无 Token 传递**：Session 不使用可传递的 bearer token，避免 token 泄露风险
- **Session 隔离**：每个 Session 绑定单一 Workspace，无法跨 Workspace 操作
- **Capability 组合**：权限由多个 Capability 组合决定，而非单一 role
- **短生命周期**：Session 默认 30 分钟 TTL，超时自动失效

### Policy Override 层级

Policy 决策采用多层级 Override，高优先级覆盖低优先级：

```
优先级（从高到低）：
┌─────────────────────────────────────────────────────────────┐
│  1. Temporary Override   ← 当前会话临时覆盖（单次操作）      │
│     (TimeWindow + SingleOperation)                          │
├─────────────────────────────────────────────────────────────┤
│  2. User Override        ← 用户级别偏好设置                  │
│     (UserId → OverrideDecision)                             │
├─────────────────────────────────────────────────────────────┤
│  3. Workspace Override   ← 工作空间级别策略                  │
│     (WorkspaceId → WorkspacePolicy)                         │
├─────────────────────────────────────────────────────────────┤
│  4. Global Policy        ← 全局默认策略                      │
│     (GlobalPolicy + DefaultRiskDecisions)                   │
└─────────────────────────────────────────────────────────────┘
```

**Override 规则**：
- **Temporary Override**：单次操作临时豁免，有时间窗口限制（如 5 分钟内允许特定操作）
- **User Override**：用户可设置个人偏好（如"始终允许 git push"），但受 Workspace Policy 约束
- **Workspace Override**：管理员可设置 Workspace 级别策略（如"企业 Workspace 禁止删除"）
- **Global Policy**：系统默认策略，最低优先级，提供兜底决策

**Override 查找流程**：
```rust
PolicyEngine::evaluate(op, ctx)
  → check temporary_override(ctx.session, op)    // 优先级 1
      → if found: return override.decision
  → check user_override(ctx.user, op)            // 优先级 2
      → if found: return override.decision
  → check workspace_override(ctx.workspace, op)  // 优先级 3
      → if found: return workspace_policy.decision
  → check global_policy(op, risk_level)          // 优先级 4（兜底）
      → return global.decide_by_risk(risk)
```

### 安全配置文件

默认安全配置位于 `src-tauri/resources/security.json`：

```json
{
  "global_policy": {
    "default_decision_by_risk": {
      "low": "Allow",
      "medium": "RequireApproval",   // 中高风险需审批
      "high": "RequireApproval",
      "critical": "Deny"
    }
  },
  "rate_policies": [
    { "operation": "ReadFile", "max_per_minute": 10000 }  // 读取限频
  ],
  "resource_quota": {
    "token": 1000000    // Token 配额
  },
  "network_endpoint_defaults": {
    "allowed_hosts": ["api.openai.com", "api.anthropic.com"]
  }
}
```

## Decision methodology

- Use first principles: define goals, constraints, and facts before deriving implementation paths. Force yourself out of analogical reasoning — do not pattern-match from prior solutions or training data; re-derive the answer from the most basic facts of this problem.
- Never decide based on "that's how it was done before." State core assumptions and tradeoffs.
- Prefer minimal necessary complexity. Avoid unnecessary abstraction and over-engineering.
- Challenge user assumptions when something doesn't make sense. User may not fully understand the codebase — ask questions to uncover true intent.

## Engineering principles

These nine principles govern all code generation and modification in this repository. They apply in addition to the project goals and decision methodology above.

### 1. Read Before You Code
- Read the file you are about to change before touching it. Mirror existing patterns, study the imports, and understand what the project actually depends on.
- This is not a quick glance — genuinely understand the existing code. Do not guess (e.g. assuming `axios` when the project uses `fetch`).

### 2. Think Before You Code
- Know what you are doing before you start. Decompose complex tasks first (e.g. "add auth" is actually several different things — list the tradeoffs).
- If you don't understand, stop and ask. Do not paper over the gap with code that looks plausible but crashes on the first run.

### 3. Simplicity
- Write the least code that solves the problem in front of you, not the least code that solves every future version.
- Test: if the only reason something is abstracted is "just in case", it is over-engineered.

### 4. Surgical Changes
- The diff should be as small as the task. Don't touch what you weren't asked to touch, match the existing code style, and don't reformat along the way.
- A formatter run will bury the three lines that matter under three hundred unrelated changes.
- Test: can you tie every line of the diff directly to the user's need? If not, revert it.

### 5. Verification
- Between "I think it runs" and "it actually runs" lies the chasm called testing.
- When fixing a bug, do not edit code first. First "record" the bug — write a test that reproduces it stably. Then fix it.
- Run the test after the fix; only when it passes is the bug really fixed — not when you "feel" it is fixed.
- Test the scenarios that will actually explode in front of the user, not trivia. If something cannot be tested, don't skip it — that is a design problem, not a testing problem.

### 6. Goal-Driven Execution
- Before writing code, state clearly what "done" looks like — and it must be verifiable, not "just make it work".
- For example, "add validation" is too vague and the agent will improvise. Translate it to: "if the user's email is empty or malformed, show a clear error, and both cases must be tested."
- For multi-step work, lay out the plan first — don't grind for an hour only to find the direction was wrong.

### 7. Debugging
- When something is broken, investigate — don't guess.
- Read the full error message and stack trace, reproduce the problem before changing anything, and change one thing at a time.
- Do not stop at the first plausible surface fix. Trace the symptom to its underlying mechanism — a patch that silences the error without addressing the root cause will resurface later, possibly as a larger failure.

### 8. Dependencies
- Every dependency is permanent code you cannot control.
- Before adding one, ask: can the standard library handle it? (e.g. `crypto.randomUUID()` vs a `uuid` package.)
- If you add it, state why — make the choice visible, don't sneak it into the manifest.

### 9. Communication
- Say what you did and why — don't just drop a chunk of code.
- Describe uncertainty precisely: "I'm not sure this library supports streaming" is good communication; "I think this should work" is not.

### 10. Adversarial Review
- Verification (Principle 5) proves the happy path and reproduces known bugs; adversarial review proactively hunts for the paths you didn't imagine.
- Review from a hostile user's perspective: what inputs would break this? Consider oversized payloads, malformed data, future/invalid timestamps, empty/null values, concurrent writes, and resource exhaustion.
- Trace each hostile input through the full path from entry to crash, not just the entry point.
- Before shipping non-trivial features, run an adversarial pass. For complex changes, prefer parallel multi-agent review (e.g. "开启 Ultracode 对本次开发进行对抗式审查").
- Periodically (every 2-3 weeks) run a project-wide adversarial review covering architecture, dependencies, code quality, and doc/code drift — surface latent tech debt before it surfaces in production.

## Layered architecture

Both the renderer process (frontend) and main process (backend) follow a layered architecture. The Rust backend uses **6 top-level layers**: `application / core / domain / infrastructure / security_kernel / shared`.

### Main process (Rust — `src-tauri/src/`)

```
application/         应用层（面向前端的 CRUD 编排 + IPC 命令入口）
  ├── session/       会话管理（commands/state/types/errors）
  ├── workspace/     工作区管理（commands/errors）
  ├── skill/         技能管理（commands/discovery/evolution_commands/state/types）
  ├── loop_engine/   Loop-Engineering 应用层（commands/builtin_patterns/types）
  └── init.rs        业务初始化编排（identity 文件生成 + builtin loop patterns seed）
core/                核心业务逻辑层（无 UI/框架依赖）
  └── agent/         AI Agent 引擎（13 子模块，见下文）
domain/              领域层（纯业务逻辑 + 领域命令）
  ├── pipeline/      Pipeline 工程（agents/runner/state/governance/interactive_film + scheduler + commands）
  ├── novel/         小说下载/解析（parser/commands/crawler/http/source）
  ├── story/         故事状态（commands/models/types）
  ├── version/       版本控制（commands/diff/models/types）
  ├── wiki/          Wiki 知识库（commands/errors/models/types）
  ├── radar/         趋势雷达（agent/commands/sources/types）
  ├── user/          用户画像（commands/learned_commands/store/types）
  ├── git/           Git 集成（commands/detector/installer/operations/parser）
  └── feedback/      反馈系统（state/store/types/errors）
infrastructure/      基础设施层（系统访问，接收 &Database，无 Tauri 依赖）
  ├── db/            SQLite + rusqlite（state + commands + 各领域 store 文件）
  ├── fs/            文件 I/O（data_dir/fs_utils/commands/types）
  ├── llm/           LLM 客户端（openai/anthropic/ollama/agnes + registry + embedding + presets + commands）
  ├── sandbox/       沙箱执行（commands/policy/types/state/heuristics）
  ├── memory/        记忆系统（commands/short_term_commands/store/state/types）
  ├── project_memory/ 项目记忆（commands/state/store/types）
  ├── tool_limits/   工具限制（commands/state/types）
  ├── mcp/           MCP 协议客户端（types/config/client/registry/state/commands）
  ├── workspace/     工作区基础设施（registry/state）
  ├── net/           网络工具（lm_ping）
  ├── secrets/       密钥管理（secrets_get/set/delete/get_all）
  ├── providers/     Provider 管理（provider_list/models/test_connection/refresh）
  ├── prompts/       提示词管理（CRUD）
  ├── settings/      系统设置（theme/log_level/git_enabled/log_files）
  ├── stats/         统计（get_stats/get_daily_activity/get_ai_stats）
  ├── notifications/ 通知（send_notification）
  ├── notify/        通知分发（notify_dispatch）
  ├── validation/    通用校验
  └── redact/        敏感信息脱敏
security_kernel/     安全内核（统一安全入口，所有敏感操作必经）
  ├── kernel.rs      SecurityKernel.execute() 唯一入口
  ├── validation/    输入校验层（path/url/endpoint/id）
  ├── policy/        策略引擎（engine/decision/global_policy/temporary_override/user_override/workspace_override）
  ├── rate_limiter/  频率限制（limiter/policy/store）
  ├── resource_manager/ 资源配额（enforcer/monitor/quota）
  ├── permission/    权限管理（session/capability/filesystem_scope/network_scope/shell_scope）
  ├── approval/      审批管理（store/token/validator）
  ├── audit/         审计事件总线（bus/event/persistence/store/tauri_emit）
  ├── plugin/        插件系统（manifest/registry/permission/security）
  ├── secrets/       内核密钥（keyring/store）
  ├── commands.rs    安全内核 IPC 命令（audit_*/approval_*/kernel_stats）
  ├── config.rs      安全配置
  ├── state.rs       SecurityKernelState
  └── types.rs       内核类型
shared/              跨层共享类型与纯函数
  ├── error.rs       统一 AppError + IpcResponse<T> + status codes
  ├── story/         故事数据类型（models/types）
  ├── version/       版本数据类型（types）
  ├── wiki/          Wiki 数据类型（models/types）
  └── utils/         纯工具函数
```

- `lib.rs` declares 6 top-level modules (application, core, domain, infrastructure, security_kernel, shared), registers commands, initializes app state.
- `#[tauri::command]` handlers live in **each layer's `commands.rs`** (e.g. `application/session/commands.rs`, `domain/novel/commands.rs`, `infrastructure/sandbox/commands.rs`, `security_kernel/commands.rs`, `core/agent/commands.rs`). Only extract params, validate, delegate. No business logic.
- `core/agent/` — AI Agent engine with 13 submodules:
  - `types` / `engine` (AgentEngine) / `approval` / `commands` (chat_send_message etc.) / `tools` (edit_tools/fs_tools/todo_tools) / `subagent` (agent/cache/executor/orchestrator/tokens/tools/types) / `prompts` / `identity` / `loop_engine` (budget/prompts/types) / `effort` (EffortLevel) / `collaboration_style` / `daily_summary` / `daily_summary_commands`
- `domain/pipeline/` — Pipeline engineering system (see "Pipeline engineering system" below)
- `infrastructure/` — system access layer (no Tauri dependencies, receives `&Database` via State):
  - `db/` — SQLite + rusqlite (NOT sqlx), WAL mode
  - `llm/` — LLM API providers (OpenAI/Anthropic/Ollama/Agnes + ProviderRegistry + embedding/RAG)
  - `sandbox/` — sandbox enforcement (commands/policy/types/state/heuristics)
  - `mcp/` — MCP (Model Context Protocol) client: config + stdio transport + tool registry + IPC commands
  - `memory/` — memory system (long-term + short-term daily summaries)
  - `fs/` — file I/O (DataDir manages all app paths)
- `security_kernel/` — unified security entry (see "Security Kernel 模型" above)
- `shared/` — pure data types and side-effect-free functions only

**Rules**:
- `#[tauri::command]` handlers (in each layer's `commands.rs`) contain NO business logic — only param extraction, validation, delegation.
- `core/agent/` does NOT depend on any `domain/` module — domain orchestrates agent, agent does not reverse-depend.
- No横向 dependencies between `domain/` modules — cross-domain orchestration goes through `application/` or `core/`.
- No Tauri dependencies in `domain/`, `core/`, or `infrastructure/` — they receive dependencies via traits and return `Result<T, AppError>`.
- `infrastructure/` only depends on `shared/`, NOT on any `domain/` or `core/agent/` (no reverse dependency).
- `shared/` only contains pure data types and side-effect-free functions — no business logic, no I/O.
- `application/init.rs` handles business initialization (extracting built-in resources, generating default identity files, seeding builtin loop patterns) — `infrastructure/fs/data_dir.rs` must NOT call business functions.
- IPC boundary is the trust seam: always validate in Rust, never trust frontend.
- All `#[command]` functions must validate inputs before delegating to `core/` or `domain/`.
- Path components from IPC (book_id, workspace_id) must be validated for traversal before use in filesystem operations.

### Renderer process (WebView — `src/`)

```
pages/ (page components) → features/{feature}/hooks/ → features/{feature}/services/ → services/ipc/
                                                                                              ↕
                                                                                       stores/ (Zustand)
                                                                                              ↕
                                                                          features/agent/services/ (LLM + tools)
```

```
src/
├── app/                    # 应用入口（App.tsx, bootstrap, lifecycle）
├── features/               # 功能模块（chat/agent/workspace/novel/...）
│   ├── agent/              # AI Agent 独立 feature
│   │   ├── components/     # Agent UI（ApprovalCard, PlanDiffReview）
│   │   ├── services/       # Agent 服务（agents/llm/state/subagent/tools/utils + legacy）
│   │   ├── store/          # Agent stores（agents-store, plan-store, todo-store）
│   │   ├── prompts/        # Agent prompts（genres/）
│   │   └── types.ts        # Agent 类型定义
│   ├── chat/               # Chat feature（components/hooks/services/store/types）
│   ├── workspace/          # Workspace management（components/hooks/services/store/types）
│   ├── story/              # Story editing（hooks/services/types/pages/novel）
│   ├── settings/           # Settings page（hooks/sections/services/types）
│   ├── wiki/               # Wiki feature
│   ├── version/            # Version control
│   ├── novel/              # Novel download/reader
│   ├── radar/              # Trend radar
│   ├── memory/             # Memory management
│   ├── skill/              # Skill management
│   ├── session/            # Session management
│   ├── knowledge/          # Knowledge base
│   ├── tools/              # Tools services（notifications）
│   ├── git/                # Git integration
│   └── loop/               # Loop engine
├── components/             # 全局公共组件
│   ├── ui/                 # shadcn/ui 基础组件（Button/Input/Modal 等）
│   ├── layout/             # 布局组件（AppLayout/AppSidebar）
│   └── shared/             # 共享原子组件（page-layout/search-input/setting-row/state）
├── services/               # 全局服务
│   ├── ipc/                # Tauri 命令包装器（ipc<T>() helper）
│   ├── llm/                # LLM proxy fetch
│   └── storage/            # 文件存储（fs）
│   └ settings.ts           # Settings 服务（loadSettings/saveSettings + AI model CRUD）
├── hooks/                  # 全局 hooks（useAsyncAction/useCopyFeedback/useIsMobile）
├── stores/                 # 全局 stores（Zustand，复杂领域状态）
├── lib/                    # 工具函数
│   ├── utils.ts            # cn() 等 Tailwind 工具
│   ├── event-bus.ts        # 统一事件总线
│   ├── constants.ts        # 全局常量（DEFAULT_PAGE/PROMPT_CATEGORIES/DEFAULT_GENRE_CONFIG）
│   ├── app-context.tsx     # 全局 app state（useReducer + Context）
│   ├── shortcuts.ts        # 快捷键核心模块
│   └── theme.tsx           # 主题管理
├── types/                  # 全局类型定义
│   ├── app.ts              # AppPage/AppState/SettingsTab
│   ├── book.ts             # Book 类型
│   ├── book-rules.ts       # BookRules 类型
│   ├── genre-profile.ts    # GenreProfile 类型
│   ├── hook.ts             # Hook 类型
│   ├── llm.ts              # LLMMessage/LLMResponse/OnStreamProgress
│   └── ...
├── locales/                # i18n
│   ├── en.ts               # 英文翻译
│   ├── zh.ts               # 中文翻译
│   ├── i18n.tsx            # I18n 实现
│   └── index.ts            # 导出
├── routes/                 # 路由（Context-based page switcher）
└── styles/                 # 全局样式（index.css）
```

**Import 路径规范**:
- `@/features/xxx` — 功能模块（如 `@/features/chat/store`, `@/features/agent/services/llm/chat-runtime`）
- `@/services/xxx` — 全局服务（如 `@/services/settings`, `@/services/ipc`, `@/services/storage/fs`）
- `@/types` — 全局类型（如 `@/types/app`, `@/types/llm`）
- `@/locales` — i18n（如 `@/locales/i18n`）
- `@/hooks` — 全局 hooks（如 `@/hooks/useAsyncAction`）
- `@/lib` — 工具函数（如 `@/lib/utils`, `@/lib/constants`, `@/lib/app-context`, `@/lib/theme`）
- `@/components/xxx` — 全局组件（如 `@/components/ui/button`, `@/components/layout/AppLayout`）

**Rules**:
- `pages/` 已移除，页面组件在 `features/{feature}/` 下（如 `features/chat/ChatPage.tsx`）
- `features/agent/` 是 AI Agent 独立功能模块，包含 services（LLM + tools）、store、prompts
- `features/settings/hooks/` 只放设置相关 hooks，全局 hooks 在 `src/hooks/`
- `lib/` 只放纯工具函数/Context，不放业务逻辑
- `services/settings.ts` 是全局 settings 服务，`features/settings/` 是设置页面功能
- `types/` 是全局类型定义，feature 内部类型在各 feature 的 `types.ts` 或 `types/` 下

## File & modularization requirements

- Large files must be split into clear modules, organized by responsibility boundaries.
- A single file that mixes multiple responsibilities (UI + state + data fetching + transformation) must be split.
- Shared capabilities must be extracted as reusable modules. No copy-paste.
- Naming must reflect responsibility. Directory structure must support quick navigation.

## Commands

- `pnpm run dev` — start Vite dev server (port 1420, strict)
- `pnpm run build` — type-check (`tsc`) then Vite build
- `cargo tauri dev` — full Tauri dev (runs `pnpm run dev` as beforeDevCommand, then launches Rust app)
- `cargo tauri build` — production build + bundle
- No lint, test, or formatter scripts are currently defined

## Key quirks

- Dev server must be on port 1420 (hardcoded in `tauri.conf.json` and `vite.config.ts`). Vite `strictPort: true` — it will fail, not pick another port.
- Vite watches `src/` but ignores `src-tauri/` (configured in `vite.config.ts`).
- Rust lib name is `mnemosyne_lib` (not `mnemosyne`) to avoid Windows bin/lib name conflict (`src-tauri/src/main.rs:5`).
- Tauri CSP is disabled (`"csp": null` in `tauri.conf.json`) — fine for dev, review before shipping.
- Capabilities system in `src-tauri/capabilities/default.json` grants permissions per-window. Add new Tauri plugin permissions there.
- Uses `pnpm` as the JS runtime (per `beforeDevCommand`/`beforeBuildCommand`), not npm/yarn.
- Package is `"type": "module"` (ESM).

## Conventions

- Rust edition 2021, `#[tauri::command]` functions live in each layer's `commands.rs` (e.g. `application/session/commands.rs`, `domain/novel/commands.rs`, `infrastructure/sandbox/commands.rs`, `security_kernel/commands.rs`, `core/agent/commands.rs`). All commands are registered in `lib.rs` via `tauri::generate_handler!`.
- Frontend calls Rust via `import { invoke } from "@tauri-apps/api/core"` then `invoke("command_name", { args })`
- TypeScript strict mode enabled (`noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`)
- React 19, JSX transform is `react-jsx` (no React import needed for JSX, but `main.tsx` imports it for `StrictMode`)
- Front-end floating layers (modal/popover/dropdown/tooltip) must portal to `document.body`, not inside `overflow-hidden` or stacking context containers
- **shadcn/ui**: Use `cn()` from `@/lib/utils` for conditional classes. Use semantic colors (`bg-primary`, `text-muted-foreground`), never raw values (`bg-blue-500`). Use `gap-*` not `space-x-*`/`space-y-*`. Use `size-*` when width=height. Add components via `pnpm dlx shadcn@latest add <component>`. Icon library: `lucide-react`.

## App data directory structure

All application data is managed by `DataDir` (`src-tauri/src/infrastructure/fs/data_dir.rs`). On first launch, all directories and default config files are created automatically.

Agent identity files (SOUL.md, CONTEXT.md, MEMORY.md) are generated by `application/init.rs` per role under `%APPDATA%/com.admin.mnemosyne/agents/<role>/`. All 16 roles (1 main agent + 15 pipeline agents) have identity files. Agent behavior prompts live in code (`domain/pipeline/agents/*.rs` for pipeline agents, `core/agent/prompts/` for main agent).

```
%APPDATA%/com.admin.mnemosyne/       (Windows)
~/Library/Application Support/com.admin.mnemosyne/  (macOS)
~/.local/share/com.admin.mnemosyne/  (Linux)
├── config.json                   # App settings (UI theme, locale, log level, AI model configs)
├── data/
│   ├── state.sqlite              # Core state (novels, chapters, sessions, messages, agents)
│   ├── feedback.sqlite           # Error events, lessons, gate evaluations, pipeline runs
│   └── logs.sqlite               # Structured logs
├── logs/                         # Rolling daily log files (mnemosyne.log.YYYY-MM-DD)
└── skills/                       # Local skill definitions
```

**Rules**:
- All paths must go through `DataDir` getters — never construct paths manually in commands or services.
- `DataDir` is stored as Tauri State and accessible from all `#[command]` handlers via `app.state::<DataDir>()`.
- Config files use `serde_json` with pretty-print for human readability.
- The database uses SQLite WAL mode (`PRAGMA journal_mode = WAL`).
- Agent identity files (SOUL.md/CONTEXT.md/MEMORY.md) are loaded from `%APPDATA%/com.admin.mnemosyne/agents/<role>/` at runtime; behavior prompts live in code (`domain/pipeline/agents/*.rs` for pipeline agents, `core/agent/prompts/` for main agent).

## Pipeline engineering system

The pipeline is orchestrated by `PipelineRunner` (`domain/pipeline/runner/pipeline_runner.rs`), managing the multi-agent novel writing flow. Agent behavior prompts live in code (`domain/pipeline/agents/*.rs`); agent identity files (SOUL.md/CONTEXT.md/MEMORY.md) are loaded from `%APPDATA%/com.admin.mnemosyne/agents/<role>/` at runtime and persisted across sessions.

### Agent roles and pipeline flow

```
Plan → Compose → Write → Audit → Revise (loop) → Reflect
  │        │        │       │         │              │
  │        │        │       │         └ reviser (if audit has critical issues)
  │        │        │       └ foundation_reviewer + state_validator (post-write gates)
  │        │        └ writer (prose generation) + length_normalizer + polisher
  │        └ composer (context assembly: semantic section selection + compressible context compilation)
  └ planner (chapter memo)
```

### All 15 pipeline agents (`domain/pipeline/agents/`)

| Agent | File | Responsibility |
|:------|:-----|:---------------|
| **architect** | `architect.rs` | Creates book structure during `novel_create` |
| **planner** | `planner.rs` | Per-chapter planning memo (intent + mustAvoid + styleEmphasis) |
| **composer** | `composer.rs` | Context assembly for writer (semantic section selection + compressible context compilation) |
| **writer** | `writer.rs` | Prose generation |
| **continuity** | `continuity.rs` | Continuity audit |
| **reviser** | `reviser.rs` | Revision when audit finds critical issues |
| **polisher** | `polisher.rs` | Style polishing |
| **length_normalizer** | `length_normalizer.rs` | Length normalization to target word count |
| **foundation_reviewer** | `foundation_reviewer.rs` | Foundation (truth file) review gate |
| **state_validator** | `state_validator.rs` | Chapter state validation gate |
| **consolidator** | `consolidator.rs` | Chapter consolidation (facts extraction + truth file update) |
| **chapter_analyzer** | `chapter_analyzer.rs` | Chapter analysis (called by consolidator) |
| **short_fiction** | `short_fiction.rs` | Short fiction pipeline runner |
| **fanfic_canon_importer** | `fanfic_canon_importer.rs` | Fanfic canon import |
| **script_storyboard** | `script_storyboard.rs` | Script/storyboard pipeline runner |

### Runner sub-modules (`domain/pipeline/runner/`)

- `pipeline_runner.rs` — main PipelineRunner orchestrator
- `chapter_review_cycle.rs` — Audit↔Revise scoring loop with best-snapshot rollback
- `chapter_state_recovery.rs` — state validation failure recovery (retry settle + degraded question construction + review note parsing)
- `chapter_truth_validation.rs` — truth file persistence validation (with state recovery + degraded marker)
- `ai_tells.rs` — AI-tells structural detection (pure rules, ported from frontend `analyzeAITells`)
- `sensitive_words.rs` — sensitive word detection (base wordlist + literal match)
- `post_write_checks.rs` — post-write deterministic checks (normalize + assert_not_empty + rule checks)
- `short_fiction_runner.rs` — short fiction pipeline runner
- `script_storyboard_runner.rs` — script/storyboard pipeline runner

### Per-agent configuration

Currently all agents receive the same standard tool set; per-agent tool filtering and token budgets are not yet implemented. Per-agent model overrides are supported via `PipelineConfig.model_overrides`.

### Feedback loop

When the auditor finds issues, `LessonTracker` records constraint lessons and appends them to the offending agent's `MEMORY.md`. Lessons are reloaded and injected into prompts on subsequent runs.

### Quality gates

Verification gates are evaluated after write via post-write checks (`runner/post_write_checks.rs` + `runner/ai_tells.rs` + `runner/sensitive_words.rs`). Gate failures trigger the revision loop (`runner/chapter_review_cycle.rs`).

### Loop-Engineering system

Loop-Engineering is split across two layers:
- `core/agent/loop_engine/` — runtime loop engine (budget/prompts/types), used for `check_budget` and sub-agent prompt injection
- `application/loop_engine/` — frontend-facing CRUD (commands/builtin_patterns/types), exposes `loop_*` IPC commands

The two layers are linked via `pattern_id` (`LoopPatternId::as_str()`). 4 builtin patterns are seeded on startup by `application/init.rs::seed_builtin_loop_patterns`.

### Garbage collection

State snapshots are saved per chapter under `<book>/story/snapshots/`. Snapshot state is managed by `domain/pipeline/state/` (store/manager/reducer/validator/bootstrap).

## Safety rules

- **High-risk operations** (data deletion, bulk writes, migrations, overwrites, irreversible changes) require explicit user consent before execution. Without consent, only read-only analysis and design is allowed. Testing and building are safe to run.
- **No silent fallbacks**: Never silently skip errors, provide default values on failure, or auto-degrade. All non-expected behavior must fail explicitly and report. Do not auto-skip unavailable models, silently swallow errors, or fabricate fallback data.
- **No `any` types**: All TypeScript must have explicit types. No `any`.
- **No hardcoded language strings**: Use i18n per project conventions.

## Git & commit rules

- Git operations are restricted to **read-only** by default: `git status`, `git log`, `git diff`, `git show`, `git branch` (read).
- All write operations (commit, push, merge, branch create/delete, reset, rebase) require explicit user consent.
- **Auto-commit on task completion**: When a user request is fully completed and the session ends, commit automatically without asking for authorization. This is the only exception to the consent rule.
- Each commit must be high-cohesion: one bugfix, one feature slice, one refactor stage, or one docs update. Unrelated changes must be split.
- Commit message must include a clear title and concise body (change summary, key verification results, risks/follow-ups).
- Before committing, run `git status` and `git diff --cached` to verify scope; exclude unrelated staged files.
- If user says "don't commit / just file changes", respect that and report uncommitted state.
- Commit only includes task-related files. No user's pre-existing changes, formatting noise, or debug files.
- **No browser-based testing**: Do not launch browsers for testing. Safe commands: build, test, lint.

### Branch management

采用 **Git Flow 简化版**（主干开发 + 功能分支）：

| 分支类型 | 命名规范 | 生命周期 | 作用 |
| :--- | :--- | :--- | :--- |
| **主分支** | `master` | 永久存在 | 线上生产环境代码。**严禁直接提交**，只能通过 PR/MR 合并。 |
| **开发分支** | `develop` | 永久存在 | 集成分支，用于最新功能联调。合并后触发测试环境部署。 |
| **功能分支** | `feature/xxx` 或 `feature/版本号-xxx` | 临时 | 从 `develop` 切出，开发完成后合并回 `develop`。 |
| **修复分支** | `hotfix/xxx` | 临时 | 从 `master` 切出，用于紧急修复线上 Bug，修复后同时合并入 `master` 和 `develop`。 |
| **发版分支** | `release/vX.Y.Z` | 临时 | 从 `develop` 切出，用于预发布测试。只允许修复 Bug，不增加新功能。测试完成后合并入 `master` 并打 Tag。 |

**分支工作流规则**：
- 功能开发从 `develop` 切出 `feature/xxx` 分支
- 推送前先 `git pull origin develop --rebase`（保持线性历史）
- 功能分支合并进 `develop` 必须通过 PR/MR
- **禁止**对 `master` 和 `develop` 使用 `git push --force`
- 功能分支整理提交可用 `--force-with-lease`

### Commit message 规范（Conventional Commits）

```text
<type>(<scope>): <subject>   # 标题行：必填
<BLANK LINE>
<body>                       # 正文：描述为什么改、怎么改（选填）
<BLANK LINE>
<footer>                     # 脚注：关闭 Issue 或 BREAKING CHANGE（选填）
```

**Type 类型**：

| Type | 含义 | 触发版本号更新 |
| :--- | :--- | :--- |
| **feat** | 新增功能/特性 | 次版本号（1.1.0） |
| **fix** | 修复 Bug | 补丁版本号（1.0.1） |
| **docs** | 仅文档修改 | 否 |
| **style** | 代码格式调整（不影响逻辑） | 否 |
| **refactor** | 代码重构（非新功能非修 Bug） | 否 |
| **perf** | 性能优化 | 补丁版本号 |
| **test** | 增加或修改测试用例 | 否 |
| **build** | 构建系统或外部依赖变更 | 否 |
| **ci** | CI 配置或脚本修改 | 否 |
| **chore** | 杂务（非 src/test 文件修改） | 否 |
| **revert** | 回滚之前的提交 | 否 |

**Subject 规则**：
- 使用祈使句，动词开头
- 不超过 50 个字符
- 首字母小写，结尾不加句号
- 示例：`fix(login): handle empty password error`

### Tag 规范（语义化版本 SemVer）

每次从 `develop` 合并入 `master` 并发布后，**必须**在 `master` 上打 Tag：
- 命名：`v[主版本号].[次版本号].[补丁版本号]`，如 `v2.1.3`
- **主版本号**：不兼容的 API 大变更
- **次版本号**：新增向下兼容的功能
- **补丁版本号**：向下兼容的 Bug 修复

## Testing standards

- New features and logic changes must have tests. Modified files that affect tests must be updated together.
- Bug fixes must include a regression test; the `it()` name should describe the bug scenario.
- Assertions must check specific values (DB fields, function params, return values), not just `toHaveBeenCalled()`.
- No "self-answering" tests: don't mock return X then assert X without exercising business logic.
- Test directories: `tests/unit/`, `tests/integration/`, `tests/system/`, `tests/regression/`, `tests/contracts/`. No cross-layer mixing.
- Files over ~350 lines or 10+ `it()` blocks must be split.
- Naming: `*.test.ts`, `*.integration.test.ts`, `*.system.test.ts`, etc.
- When modifying tests, reuse existing helpers/fixtures. Do not rebuild mock frameworks for the same topic.

## Architecture constraints

- Front-end floating layers must portal to `document.body`. No attaching to `overflow-hidden` containers or elements that create stacking contexts.
- Front-end state management follows project conventions (Zustand for complex state like workflow, useReducer + Context for global app state).
- File organization must reflect responsibility boundaries. No single files mixing UI, state, data fetching, and transformation.
- Public capabilities must be extracted as reusable modules, not copy-pasted.
- Payload and IPC field semantics must share the same type or normalization function across frontend and Rust. No mismatched read/write semantics.
- Flows that "create a record then submit a task" must complete pre-validation first. If task submission can fail, provide explicit compensation/rollback. No zombie data.

## IPC conventions (CRITICAL)

**Naming**: Tauri auto-converts camelCase (JS) ↔ snake_case (Rust). Frontend MUST use camelCase for all argument keys. Rust commands use snake_case parameters.

```typescript
// ✅ Correct - frontend uses camelCase
await ipc("agent_send_message", { sessionId, content });

// ❌ Wrong - snake_case in frontend
await ipc("agent_send_message", { session_id: sessionId, content });
```

**Response envelope**: All `#[command]` functions return `Result<IpcResponse<T>, AppError>`. Use `IpcResponse::ok(data)` for success, `AppError` constructors for errors.

**Status codes**: Use the appropriate status code from `src-tauri/src/shared/error.rs`. All `AppError` constructors are available: `bad_request`, `unauthorized`, `forbidden`, `not_found`, `conflict`, `internal`, etc.

**Frontend IPC helpers**: Use `ipc<T>()` for data responses, `ipcVoid()` for void responses. Never use `ipc<void>()` which throws on null data.

## i18n conventions (CRITICAL)

**No hardcoded strings**: Every user-visible string MUST use i18n. Add keys to both `src/locales/en.ts` and `src/locales/zh.ts`.

**Checklist for new features**:
- [ ] All UI text uses `t.keypath` or `t.section.key`
- [ ] Both `en.ts` and `zh.ts` have the new keys
- [ ] Error messages shown to users are localized
- [ ] Placeholder text is localized
- [ ] Button labels are localized
- [ ] Dialog titles/descriptions are localized

**Pattern**: Add keys under the relevant section (e.g., `modelSettings`, `agentChat`, `settings`). Keep keys descriptive and nested.

## Code comment standards

所有源代码文件必须遵循统一的注释风格规范，确保代码可读性和维护性。

### 注释风格要求

1. **禁止借鉴其他项目注释**：不得包含 "From xxx", "Based on xxx", "Similar to xxx", "参考 xxx", "对照 xxx" 等引用其他项目的注释
2. **使用中文注释**：所有注释必须使用中文
3. **简洁直观**：注释应简洁明了，直接说明代码用途

### Rust 文件注释格式

**文件头部注释**：
```rust
//! ═══════════════════════════════════════════════════════════════════════════
//! 模块名称 - 模块简短描述
//! ═══════════════════════════════════════════════════════════════════════════

/// 函数/结构体简短描述
fn function_name() {}
```

**区块分隔注释**：
```rust
// ── 区块名称 ────────────────────────────────────────────────────────────────
```

**文档注释**：
```rust
/// 函数简短描述
///
/// # 参数
/// - `param`: 参数说明
///
/// # 返回值
/// 返回值说明
///
/// # 示例
/// ```
/// let result = function_name(param);
/// ```
fn function_name(param: &str) -> Result<()> {}
```

### TypeScript 文件注释格式

**文件头部注释**：
```typescript
/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 模块名称 - 模块简短描述
 * ═══════════════════════════════════════════════════════════════════════════
 */
```

**区块分隔注释**：
```typescript
// ── 区块名称 ────────────────────────────────────────────────────────────────
```

**函数/组件注释**：
```typescript
/**
 * 函数简短描述
 * @param param - 参数说明
 * @returns 返回值说明
 */
function functionName(param: string): Result {}
```

### 注释风格示例

**Rust 示例**：
```rust
//! ═══════════════════════════════════════════════════════════════════════════
//! 进程监控 - 实时监控系统进程状态
//! ═══════════════════════════════════════════════════════════════════════════

use sysinfo::System;

// ── 进程信息结构体 ────────────────────────────────────────────────────────────

/// 进程信息
pub struct ProcessInfo {
    /// 进程名称
    pub name: String,
    /// 进程 ID
    pub pid: u32,
    /// CPU 使用率（百分比）
    pub cpu_usage: f64,
    /// 内存占用（MB）
    pub memory_mb: f64,
}
```

**TypeScript 示例**：
```typescript
/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 进程监控组件 - 实时显示进程状态
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useState } from "react";

// ── 类型定义 ────────────────────────────────────────────────────────────────

/** 进程信息 */
interface ProcessInfo {
  name: string;
  pid: number;
  cpuUsage: number;
  memoryMb: number;
}
```
