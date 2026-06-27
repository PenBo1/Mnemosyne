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

- Use first principles: define goals, constraints, and facts before deriving implementation paths.
- Never decide based on "that's how it was done before." State core assumptions and tradeoffs.
- Prefer minimal necessary complexity. Avoid unnecessary abstraction and over-engineering.
- Challenge user assumptions when something doesn't make sense. User may not fully understand the codebase — ask questions to uncover true intent. ## Layered architecture

Both the renderer process (frontend) and main process (backend) follow layered architecture:

**Renderer process (WebView — `src/`)**:
```
UI Layer (pages/components) → Hook Layer (hooks/) → Service Layer (services/) → IPC Layer (lib/ipc/)
                                                                        ↕
                                                                 State (lib/store.tsx, stores/)
```
- `pages/` — page-level components, one per route/view. Must not contain IPC calls or business logic.
- `router/` — Context-based page switcher (no URL router)
- `components/` — reusable UI (ui/ for shadcn, layout/ for structure, shared/ for atoms)
- `hooks/` — bridge between services and React state (e.g. `useNovels`, `usePrompts`, `useTrends`)
- `services/` — IPC call wrappers and orchestration logic, no React dependencies
- `lib/ipc/` — `invoke()` wrappers with common `ipc<T>()` helper
- `lib/store.tsx` — `useReducer` + Context for global app state (page navigation, settings tab)
- `stores/` — Zustand stores for complex domain state (e.g. workspace)

**Main process (Rust — `src-tauri/src/`)**:
```
Command Layer (app/commands/) → Domain Layer (domain/) + Infrastructure Layer (infra/)
                                        ↓
                               System Layer (infra/db/, infra/sandbox/)
```
- `lib.rs` registers commands, initializes app state (AppState with db)
- `app/commands/` — thin `#[tauri::command]` handlers: extract params, validate, delegate to domain/infra, return
- `app/state.rs` — `AppState` struct holding all shared state (db, providers, sessions, etc.)
- `domain/` — business logic as pure functions/structs, no Tauri dependencies. Sub-modules: `agents/`, `pipeline/`, `session/`, `story/`, `wiki/`, `version/`, `radar/`, `novel/`, `harness/`
- `infra/` — system access layer: `db/` (SQLite), `llm/` (providers), `sandbox/` (security), `memory.rs`, `feedback.rs`, `mcp.rs`, `rag.rs`, `skill/`
- `errors/` — unified `AppError` type (`app_error.rs`), `IpcResponse<T>` envelope (`ipc.rs`), status codes (`status.rs`)
- `middleware/` — cross-cutting concerns (e.g. `logging.rs` for structured tracing)

**Rules**:
- No business logic in `#[command]` handlers — they extract params, validate, delegate to domain, and return.
- No Tauri dependencies in `domain/` — domain structs receive dependencies via traits and return `Result<T, AppError>`.
- No Tauri dependencies in `infra/` — infra modules receive `&Database` and return `Result<T, AppError>`.
- IPC boundary is the trust seam: always validate in Rust, never trust frontend.
- All `#[command]` functions must validate inputs before delegating to domain/infra.
- Path components from IPC (book_id, workspace_id) must be validated for traversal before use in filesystem operations.

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

Agent configurations and harness configs are **embedded in code** (`domain/harness/agent_configs.rs`, `domain/harness/global_config.rs`). No file-based loading — configs live in the binary only.

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
- Agent configs are compiled into the binary — never load from files at runtime.

## Harness engineering system

The harness system manages the 8-agent novel writing pipeline. All agent configuration is JSON-driven and **embedded in code**.

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

### Agent configuration format

Each agent has a JSON config in `config/agents/` with:
- `prompt_template`: The system prompt (replaces hardcoded prompts)
- `tools`: Allowed/denied tools for this agent
- `context`: Required/optional context sections and token budget
- `output`: Expected output format and validation rules
- `constraints`: must_do, must_not_do, style_rules
- `quality_standards`: Gate IDs and acceptance criteria
- Role-specific fields: `audit_config`, `revision_config`, `extraction_config`, etc.

### Feedback loop

When the auditor finds issues:
1. Each Critical/Warning issue is recorded as an `error_event` in SQLite
2. When an error type reaches its threshold (defined in `GlobalHarnessConfig` → `feedback_rules`), a `constraint_lesson` is generated
3. Active lessons are injected into agent prompts via `ContextBuilder`

### Quality gates

Quality gates are evaluated after audit. If gates fail, the revision loop is triggered. Gate types:
- `ScoreThreshold`: Audit score must meet minimum
- `IssueCount`: Critical issues must not exceed maximum
- `WordCountRange`: Word count must be within range
- `ForbiddenPattern`: Content must not contain forbidden phrases
- `CompletenessCheck`: Required fields must be present
- `DimensionScore`: Specific audit dimension must pass

### Garbage collection

GC runs automatically every N chapters (configurable in `harness.json` → `gc_policy`):
- Cleans stale snapshots older than `stale_snapshot_days`
- Compacts state by deduplicating facts and trimming summaries

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
