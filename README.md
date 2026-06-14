# Mnemosyne

AI 小说写作与创作管理桌面应用。基于 Tauri v2 + React 19 + TypeScript + Rust 构建。

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri v2 |
| 前端 | React 19, TypeScript, Vite, Tailwind CSS v4 |
| UI 组件 | shadcn/ui (Radix UI) |
| 状态管理 | Zustand (领域状态) + React Context (全局 UI 状态) |
| 后端 | Rust (Edition 2021) |
| 数据库 | SQLite (rusqlite, WAL 模式) |
| AI 集成 | OpenAI API, Tauri LLM Provider |

## 项目结构

```
├── src/                          # 前端 (React + TypeScript)
│   ├── components/
│   │   ├── ui/                   # shadcn/ui 组件
│   │   ├── layout/               # 布局组件 (AppSidebar, AppLayout)
│   │   └── providers/            # Context providers (Theme, I18n)
│   ├── pages/                    # 页面组件 (按路由组织)
│   │   └── settings/             # 设置子页面
│   ├── hooks/                    # React hooks (桥接 services 和 stores)
│   ├── services/                 # IPC 调用封装 (无 React 依赖)
│   ├── stores/                   # Zustand 状态管理
│   ├── lib/
│   │   ├── ipc/                  # Tauri invoke 封装
│   │   ├── locales/              # i18n 翻译文件 (en, zh)
│   │   └── app-context.tsx       # 全局 UI 状态 (React Context)
│   ├── types/                    # TypeScript 类型定义
│   ├── constants/                # 常量
│   └── router/                   # 页面路由 (Context-based)
│
├── src-tauri/                    # 后端 (Rust)
│   ├── src/
│   │   ├── app/
│   │   │   └── commands/         # Tauri IPC 命令
│   │   ├── domain/               # 业务逻辑层
│   │   │   ├── agent/            # AI Agent 循环
│   │   │   ├── pipeline/         # 小说写作 Pipeline
│   │   │   ├── harness/          # 质量门禁 & 约束
│   │   │   └── tools/            # 工具注册
│   │   ├── infra/
│   │   │   ├── db/               # SQLite 数据库
│   │   │   ├── llm/              # LLM Provider
│   │   │   ├── sandbox/          # 沙箱安全
│   │   │   └── skill/            # 技能管理
│   │   ├── errors/               # 统一错误处理
│   │   └── middleware/           # 日志等中间件
│   └── capabilities/             # Tauri 权限配置
```

## 分层架构

**前端渲染进程：**
```
UI (pages/components) → Hooks → Services → IPC (lib/ipc/)
                                      ↕
                               State (stores/, app-context)
```

**后端主进程：**
```
Commands → Services → System (db/, OS access)
```

核心规则：
- 页面不直接调用 IPC，通过 services → hooks 传递
- Services 不依赖 React，纯函数封装
- 所有前端输入在 Rust 端校验，不信任前端数据

## 开发命令

```bash
# 前端开发 (仅 Vite)
bun run dev

# 完整 Tauri 开发 (前端 + Rust)
cargo tauri dev

# 类型检查 + 构建
bun run build

# Rust 编译检查
cd src-tauri && cargo check
```

## 主要功能

- **AI 对话** — 与 AI 助手对话，支持流式响应和工具调用
- **小说管理** — 创建、管理小说项目，章节写作
- **写作 Pipeline** — 8 个专职 Agent 协作：建筑师、规划师、编排师、写手、审计员、修订者、观察者、反射器
- **市场雷达** — 扫描小说平台排行榜，AI 分析市场趋势
- **知识库** — 管理世界观、角色、剧情等创作知识
- **技能系统** — 可扩展的 AI 技能插件
- **安全沙箱** — 限制文件系统和网络访问

## 环境要求

- [Node.js](https://nodejs.org/) (Bun 推荐)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)
