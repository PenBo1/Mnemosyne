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
                                                                 State (lib/store.tsx)
```
- `pages/` — page-level components, one per route/view
- `router/` — Context-based page switcher (no URL router)
- `components/` — reusable UI (ui/ for shadcn, layout/ for structure, shared/ for atoms)
- `hooks/` — bridge between services and React state
- `services/` — business logic, validation, orchestration
- `lib/ipc/` — `invoke()` wrappers with common `ipc<T>()` helper
- `lib/store.tsx` — `useReducer` + Context for global state

**Main process (Rust — `src-tauri/src/`)**:
```
Command Layer (commands/) → Service Layer (services/) → System Layer (db/, OS access)
```
- `lib.rs` registers commands; each `#[command]` is thin (validate input, delegate to service)
- `commands/` — thin `#[tauri::command]` handlers: parse, validate, delegate to service, return
- `services/` — business logic as pure functions, no Tauri dependencies
- `db/` — database access layer (to be expanded with SQLite/ORM)
- `errors.rs` — unified `AppError` type with code + message, convertible to `String` for IPC

**Rules**:
- No business logic in `#[command]` handlers — they parse, validate, delegate, and return.
- No UI in `src/lib/` — it must be framework-agnostic.
- IPC boundary is the trust seam: always validate in Rust, never trust frontend.

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
