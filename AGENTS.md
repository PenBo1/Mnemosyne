# AGENTS.md

## Applicable scope

This specification applies to all directories and files in this repository.
If subdirectories contain their own AGENTS.md, sub-level rules may supplement but must never weaken the constraints in this file.

## Project overview

Tauri v2 desktop app with React 19 + TypeScript frontend (Vite) and Rust backend. Early stage (v0.1.0), scaffolded from the official Tauri template. Project name: Thalia (comedic/muse goddess).

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

Both the renderer process (frontend) and main process (backend) follow a five-layer architecture: **core / features / infrastructure / ipc (or shared) / shared**.

### Main process (Rust — `src-tauri/src/`)

```
ipc/                 IPC 层（Tauri 命令入口，类型安全契约）
  ├── core/          核心业务逻辑（agent 引擎、interaction 编排、state、init）
  │     ├── agent/   AI Agent 核心决策引擎（14 子模块）
  │     └── interaction/  编排层（session ↔ pipeline 桥接）
  ├── features/      功能模块层（story/session/version/wiki/novel/radar/user_profile/skill_manager）
  ├── infrastructure/ 基础设施层（db/llm_client/sandbox/file_storage/state_store/ai_services/middleware/utils）
  └── shared/        跨层共享类型与纯函数（含 errors 错误处理）
```

- `lib.rs` declares 5 top-level modules (core, features, infrastructure, ipc, shared), registers commands, initializes app state (AppState with db)
- `ipc/commands/` — thin `#[tauri::command]` handlers. Only extract params, validate, delegate. No business logic.
- `core/` — core business logic with no UI/framework dependencies
  - `core/agent/` — AI Agent engine (base, pipeline, loop_engine, identity, main_agent, chat_loop, prompts, tools, etc.)
  - `core/interaction/` — orchestration layer (session ↔ pipeline bridge)
  - `core/state.rs` — AppState (app_handle, db, provider_registry, skill_manager, sandbox, memory_store, feedback_store, mcp_server, scheduler, sessions, agent_states, main_agent_states)
  - `core/init.rs` — business initialization orchestration
- `features/` — feature modules (pure functions/structs, no Tauri dependencies): `story/`, `session/`, `version/`, `wiki/`, `novel/`, `radar/`, `user_profile/`, `skill_manager/`
- `infrastructure/` — system access layer (no Tauri dependencies, receives `&Database`):
  - `db/` — SQLite + sqlx (store files per business domain)
  - `llm_client/` — LLM API providers (OpenAI/Anthropic/Ollama/Agnes + ProviderRegistry)
  - `sandbox/` — sandbox enforcement + security (policy, enforce, fs_sandbox, exec_sandbox, net_sandbox, timeout, security)
  - `file_storage/` — file I/O (data_dir, fs_utils, epub, secrets)
  - `state_store/` — state stores (memory, feedback, gc)
  - `ai_services/` — AI services (mcp, rag, token_budget, output_validator, proxy_fetch)
  - `middleware/` — cross-cutting concerns (logging)
  - `utils/` — utility functions (text_utils with count_words)
- `shared/` — cross-layer shared types and side-effect-free functions
  - `shared/errors/` — unified `AppError` type (`app_error.rs`), `IpcResponse<T>` envelope (`ipc.rs`), status codes (`status.rs`)
  - `shared/memory/`, `shared/text/`, `shared/version/`, `shared/wiki/` — pure data types

**Rules**:
- `ipc/commands/` handlers contain NO business logic — only param extraction, validation, delegation.
- `core/agent/` does NOT depend on any `features/` module — features orchestrate agent, agent does not reverse-depend.
- No横向 dependencies between `features/` modules — cross-feature orchestration goes through `core/interaction/` or `ipc/commands/`.
- No Tauri dependencies in `features/`, `core/`, or `infrastructure/` — they receive dependencies via traits and return `Result<T, AppError>`.
- `infrastructure/` only depends on `shared/`, NOT on any `features/` or `core/agent/` (no reverse dependency).
- `shared/` only contains pure data types and side-effect-free functions — no business logic, no I/O.
- `core/init.rs` handles business initialization (extracting built-in resources, generating default identity files) — `infrastructure/file_storage/data_dir.rs` must NOT call business functions.
- IPC boundary is the trust seam: always validate in Rust, never trust frontend.
- All `#[command]` functions must validate inputs before delegating to `core/` or `features/`.
- Path components from IPC (book_id, workspace_id) must be validated for traversal before use in filesystem operations.

### Renderer process (WebView — `src/`)

```
pages/ (page components) → features/{feature}/hooks/ → features/{feature}/services/ → infrastructure/api/
                                                                                              ↕
                                                                                       stores/ (Zustand)
                                                                                              ↕
                                                                          core/agent/ (pure logic kernel)
```

- `pages/` — page-level components, one per route/view. Must not contain IPC calls or business logic. Only call hooks and compose components.
- `routes/` — Context-based page switcher (no URL router)
- `features/` — feature modules (each feature is self-contained, communicates via contracts):
  - `features/chat/` — chat feature (components/, hooks/, services/)
  - `features/workspace/` — workspace management (components/, hooks/, services/)
  - `features/story/` — story editing (hooks/, services/)
  - `features/settings/` — settings page (hooks/, services/)
  - `features/wiki/`, `features/version/`, `features/loop/`, `features/novel/`, `features/radar/`, `features/memory/`, `features/sandbox/`, `features/skill/`, `features/stats/`, `features/session/`, `features/knowledge/`, `features/tools/` — other features
- `core/` — core layer (UI-independent pure logic)
  - `core/agent/` — AI Agent state machine and decision logic (stream-protocol, tool-protocol, session-lifecycle) — no React/Tauri/IPC dependencies, shared by hooks and stores
  - `core/memory/` — frontend memory management (async action helpers)
- `infrastructure/` — infrastructure layer (external service adapters)
  - `infrastructure/api/` — Tauri command wrappers with `ipc<T>()` helper (only IPC exit point)
  - `infrastructure/event_bus/` — unified event bus (all Tauri `listen()` must go through here)
- `shared/` — shared layer (cross-module utilities)
  - `shared/types/` — cross-layer shared types (single source of truth, no type exports from services/hooks)
  - `shared/constants/` — centralized constants
  - `shared/utils/` — utility functions
  - `shared/locales/` — i18n translation files (en.ts, zh.ts)
  - `shared/i18n.tsx` — I18n full implementation
  - `shared/theme.tsx` — Theme full implementation
  - `shared/app-context.tsx` — `useReducer` + Context for global app state (page navigation, settings tab)
  - `shared/settings.ts` — settings utilities
- `components/` — common UI components (business-agnostic)
  - `components/ui/` — shadcn/ui base components (Button, Input, Modal, etc.)
  - `components/layout/` — layout components (AppLayout, AppSidebar, etc.)
  - `components/shared/` — shared atomic components
- `hooks/` — global custom hooks (business-agnostic, e.g. `useCopyFeedback`, `useIsMobile`). Feature-specific hooks live in `features/{feature}/hooks/`.
- `stores/` — Zustand stores for complex domain state. `stores/agent/` (chat-store.ts + main-agent-store.ts) for AI Agent state.
- `styles/` — global styles (index.css)

## File & modularization requirements

- Large files must be split into clear modules, organized by responsibility boundaries.
- A single file that mixes multiple responsibilities (UI + state + data fetching + transformation) must be split.
- Shared capabilities must be extracted as reusable modules. No copy-paste.
- Naming must reflect responsibility. Directory structure must support quick navigation.

## Commands

- `bun run dev` — start Vite dev server (port 1420, strict)
- `bun run build` — type-check (`tsc`) then Vite build
- `cargo tauri dev` — full Tauri dev (runs `bun run dev` as beforeDevCommand, then launches Rust app)
- `cargo tauri build` — production build + bundle
- No lint, test, or formatter scripts are currently defined

## Key quirks

- Dev server must be on port 1420 (hardcoded in `tauri.conf.json` and `vite.config.ts`). Vite `strictPort: true` — it will fail, not pick another port.
- Vite watches `src/` but ignores `src-tauri/` (configured in `vite.config.ts`).
- Rust lib name is `thalia_lib` (not `thalia`) to avoid Windows bin/lib name conflict (`src-tauri/src/main.rs:5`).
- Tauri CSP is disabled (`"csp": null` in `tauri.conf.json`) — fine for dev, review before shipping.
- Capabilities system in `src-tauri/capabilities/default.json` grants permissions per-window. Add new Tauri plugin permissions there.
- Uses `bun` as the JS runtime (per `beforeDevCommand`/`beforeBuildCommand`), not npm/yarn.
- Package is `"type": "module"` (ESM).

## Conventions

- Rust edition 2021, `#[tauri::command]` functions go in `src-tauri/src/lib.rs`
- Frontend calls Rust via `import { invoke } from "@tauri-apps/api/core"` then `invoke("command_name", { args })`
- TypeScript strict mode enabled (`noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`)
- React 19, JSX transform is `react-jsx` (no React import needed for JSX, but `main.tsx` imports it for `StrictMode`)
- Front-end floating layers (modal/popover/dropdown/tooltip) must portal to `document.body`, not inside `overflow-hidden` or stacking context containers
- **shadcn/ui**: Use `cn()` from `@/lib/utils` for conditional classes. Use semantic colors (`bg-primary`, `text-muted-foreground`), never raw values (`bg-blue-500`). Use `gap-*` not `space-x-*`/`space-y-*`. Use `size-*` when width=height. Add components via `bunx --bun shadcn@latest add <component>`. Icon library: `lucide-react`.

## App data directory structure

All application data is managed by `DataDir` (`src-tauri/src/infra/data_dir.rs`). On first launch, all directories and default config files are created automatically.

Agent identity files (SOUL.md, CONTEXT.md, MEMORY.md) are generated by `core/init.rs` per role under `%APPDATA%/com.admin.mnemosyne/agents/<role>/`. Agent behavior prompts live in code (`core/agent/prompts/`).

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
- `DataDir` is stored in `AppState` and accessible from all `#[command]` handlers via `state.data_dir`.
- Config files use `serde_json` with pretty-print for human readability.
- The database uses SQLite WAL mode (`PRAGMA journal_mode = WAL`).
- Agent identity files (SOUL.md/CONTEXT.md/MEMORY.md) are loaded from `%APPDATA%/com.admin.mnemosyne/agents/<role>/` at runtime; behavior prompts live in code (`core/agent/prompts/`).

## Pipeline engineering system

The pipeline is orchestrated by `PipelineRunner` (`core/agent/pipeline/runner.rs`), managing the 8-agent novel writing flow. Agent behavior prompts live in code (`core/agent/prompts/`); agent identity files (SOUL.md/CONTEXT.md/MEMORY.md) are loaded from `%APPDATA%/com.admin.mnemosyne/agents/<role>/` at runtime and persisted across sessions.

### Agent roles and pipeline flow

```
Plan → Compose → Write → Audit → Revise (loop) → Reflect
  │        │        │       │         │              │
  │        │        │       │         │              └ reflector + observer
  │        │        │       │         └ reviser (if audit has critical issues)
  │        │        │       └ auditor (10 quality dimensions)
  │        │        └ writer (prose generation)
  │        └ composer (context assembly)
  └ planner (chapter memo)
```

Additional standalone agents:
- **architect**: Creates book structure during `novel_create`
- **observer**: Extracts facts from chapters (called by reflector)

### Per-agent configuration

Currently all agents receive the same standard tool set; per-agent tool filtering and token budgets are not yet implemented. Per-agent model overrides are supported via `PipelineConfig.model_overrides`.

### Feedback loop

When the auditor finds issues, `LessonTracker` (`core/agent/lesson_tracker/`) records constraint lessons and appends them to the offending agent's `MEMORY.md`. Lessons are reloaded and injected into prompts on subsequent runs.

### Quality gates

Verification gates are evaluated after write via `VerificationPipeline` (`core/agent/verification/`). Gate failures trigger the revision loop.

### Garbage collection

State snapshots are saved per chapter under `<book>/story/snapshots/`. GC of stale snapshots is handled by `infrastructure/state_store/gc/`.

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

**Status codes**: Use the appropriate status code from `src-tauri/src/errors/status.rs`. All `AppError` constructors are available: `bad_request`, `unauthorized`, `forbidden`, `not_found`, `conflict`, `internal`, etc.

**Frontend IPC helpers**: Use `ipc<T>()` for data responses, `ipcVoid()` for void responses. Never use `ipc<void>()` which throws on null data.

## i18n conventions (CRITICAL)

**No hardcoded strings**: Every user-visible string MUST use i18n. Add keys to both `src/lib/locales/en.ts` and `src/lib/locales/zh.ts`.

**Checklist for new features**:
- [ ] All UI text uses `t.keypath` or `t.section.key`
- [ ] Both `en.ts` and `zh.ts` have the new keys
- [ ] Error messages shown to users are localized
- [ ] Placeholder text is localized
- [ ] Button labels are localized
- [ ] Dialog titles/descriptions are localized

**Pattern**: Add keys under the relevant section (e.g., `modelSettings`, `agentChat`, `settings`). Keep keys descriptive and nested.
