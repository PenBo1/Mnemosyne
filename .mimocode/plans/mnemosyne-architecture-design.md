# Mnemosyne 完整架构设计方案

> 基于 OpenAI Codex / Hermes Agent / InkOS 三大项目的深度调研，结合 Mnemosyne 现有代码库（Tauri v2 + Rust + React 19），输出全面的 Harness工程 + Agent工程设计方案。

---

## 目录

1. [系统总览与架构图](#1-系统总览与架构图)
2. [Harness流水线设计](#2-harness流水线设计)
3. [Agent工程设计](#3-agent工程设计)
4. [应用数据目录结构](#4-应用数据目录结构)
5. [数据库设计](#5-数据库设计)
6. [记忆系统设计](#6-记忆系统设计)
7. [上下文管理设计](#7-上下文管理设计)
8. [工具系统设计](#8-工具系统设计)
9. [配置系统设计](#9-配置系统设计)
10. [安全与沙箱设计](#10-安全与沙箱设计)
11. [前端架构设计](#11-前端架构设计)
12. [状态管理设计](#12-状态管理设计)
13. [质量门与反馈循环](#13-质量门与反馈循环)
14. [实施路线图](#14-实施路线图)

---

## 1. 系统总览与架构图

### 1.1 系统定位

Mnemosyne 是一款 Tauri v2 桌面应用，核心能力是 **AI驱动的小说创作流水线**。8个专业Agent协作完成从建书到发布的全流程。

### 1.2 整体架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Mnemosyne Desktop App                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────────────────────┐  ┌──────────────────────────────┐ │
│  │     Frontend (WebView)       │  │     Backend (Rust Core)      │ │
│  │                              │  │                              │ │
│  │  ┌────────┐  ┌──────────┐   │  │  ┌────────────────────────┐  │ │
│  │  │ Pages  │  │Components│   │  │  │   Command Layer        │  │ │
│  │  └───┬────┘  └─────┬────┘   │  │  │   (#[tauri::command])  │  │ │
│  │      │              │        │  │  └──────────┬─────────────┘  │ │
│  │  ┌───▼────┐  ┌─────▼────┐   │  │  ┌──────────▼─────────────┐  │ │
│  │  │ Hooks  │  │  Stores  │   │  │  │   Service Layer        │  │ │
│  │  └───┬────┘  └─────┬────┘   │  │  │   (pure functions)     │  │ │
│  │      │              │        │  │  └──────────┬─────────────┘  │ │
│  │  ┌───▼──────────────▼────┐   │  │  ┌──────────▼─────────────┐  │ │
│  │  │   Service Layer       │   │  │  │   Domain Layer         │  │ │
│  │  │   (IPC wrappers)      │   │  │  │                        │  │ │
│  │  └───────────┬───────────┘   │  │  │  ┌──────────────────┐  │  │ │
│  │              │               │  │  │  │ Harness Engine   │  │  │ │
│  │  ┌───────────▼───────────┐   │  │  │  │ (8 agents)       │  │  │ │
│  │  │   IPC Layer           │   │  │  │  └──────────────────┘  │  │ │
│  │  │   (invoke wrappers)   │◄──┼──┼──┤  ┌──────────────────┐  │  │ │
│  │  └───────────────────────┘   │  │  │  │ Agent System     │  │  │ │
│  │                              │  │  │  │ (BaseAgent trait) │  │  │ │
│  └──────────────────────────────┘  │  │  └──────────────────┘  │  │ │
│                                    │  │  ┌──────────────────┐  │  │ │
│       Tauri IPC Bridge             │  │  │ Tool System      │  │  │ │
│       (camelCase ↔ snake_case)     │  │  │ (registry+exec)  │  │  │ │
│                                    │  │  └──────────────────┘  │  │ │
│                                    │  └──────────┬─────────────┘  │ │
│                                    │             │                │ │
│                                    │  ┌──────────▼─────────────┐  │ │
│                                    │  │   Infrastructure       │  │ │
│                                    │  │   ┌─────┐ ┌─────────┐  │  │ │
│                                    │  │   │ DB  │ │ LLM     │  │  │ │
│                                    │  │   │     │ │ Provider│  │  │ │
│                                    │  │   └─────┘ └─────────┘  │  │ │
│                                    │  │   ┌─────┐ ┌─────────┐  │  │ │
│                                    │  │   │State│ │Sandbox  │  │  │ │
│                                    │  │   │File │ │Enforcer │  │  │ │
│                                    │  │   └─────┘ └─────────┘  │  │ │
│                                    │  └────────────────────────┘  │ │
│                                    └──────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.3 数据流总览

```
用户输入
  │
  ▼
┌──────────────┐
│  IPC Command  │ ── validate ──► Service Layer
└──────────────┘
       │
       ▼
┌──────────────┐
│ Harness Engine│ ── orchestrates ──► 8-Stage Pipeline
└──────────────┘
       │
       ▼
┌──────────────┐
│ Agent System  │ ── executes ──► BaseAgent::execute()
└──────────────┘
       │
       ├──► LLM Provider (OpenAI/Ollama/Agnes)
       ├──► Tool Registry (file/novel/search tools)
       ├──► State Manager (file-based state)
       └──► Memory System (SQLite + FTS)
              │
              ▼
       Structured Output
              │
              ▼
       State Reducer (apply delta)
              │
              ▼
       Quality Gates (audit + feedback)
              │
              ▼
       IPC Response ◄── Frontend Update
```

---

## 2. Harness流水线设计

### 2.1 流水线总览（7阶段）

参考 InkOS 的 7阶段流水线，结合 Thalia 的 8-agent 设计：

```
┌─────────┐    ┌──────────┐    ┌─────────┐    ┌───────────┐
│  Plan   │───►│ Compose  │───►│  Write  │───►│  Settle   │
│(Planner)│    │(Composer)│    │(Writer) │    │(Observer+ │
│         │    │          │    │         │    │ Reflector)│
└─────────┘    └──────────┘    └─────────┘    └─────┬─────┘
                                                      │
                                                      ▼
                                                ┌───────────┐    ┌──────────┐
                                                │   Audit   │───►│  Revise  │
                                                │(Auditor)  │    │(Reviser) │
                                                └─────┬─────┘    └────┬─────┘
                                                      │               │
                                                      │  ┌────────────┘
                                                      │  │ (loop if critical issues)
                                                      ▼  ▼
                                                ┌───────────┐
                                                │  Persist  │
                                                │ (Snapshot)│
                                                └───────────┘
```

### 2.2 各阶段详细设计

#### Stage 1: Plan（规划）

| 属性 | 值 |
|------|-----|
| **Agent** | Planner |
| **输入** | StoryState（当前状态、钩子、事实、摘要） |
| **输出** | `ChapterIntent`（goal, must_keep, must_avoid, focus_points, hook_agenda） |
| **LLM温度** | 0.3（低温度，确定性规划） |
| **Token预算** | 2048 |
| **验证** | JSON Schema校验，字段完整性检查 |

**输出Schema**:
```json
{
  "chapter": 5,
  "goal": "揭示反派身份，推进主线冲突",
  "must_keep": ["反派面具掉落", "主角内心挣扎"],
  "must_avoid": ["引入新角色", "时间跳跃"],
  "focus_points": ["情感张力", "悬念设置"],
  "hook_agenda": {
    "open": [{"name": "神秘信件", "description": "信件内容暗示更大阴谋"}],
    "advance": [{"name": "师徒矛盾", "description": "师父的真实意图浮出水面"}],
    "resolve": [],
    "defer": [{"name": "身世之谜", "description": "推迟到第8章"}]
  }
}
```

#### Stage 2: Compose（编排）

| 属性 | 值 |
|------|-----|
| **Agent** | Composer |
| **输入** | ChapterIntent + StoryState + BookRules + GenreProfile |
| **输出** | `ContextPackage`（selected_context + rule_stack） |
| **LLM温度** | 0.2（最低温度，精确选择） |
| **Token预算** | 4096 |
| **验证** | 上下文来源可追溯性检查 |

**核心职责**：
- 从 truth files 中选择与本章相关的上下文
- 组装分层规则栈（硬规则/软规则/诊断规则）
- 生成 `ChapterTrace`（为什么选择这些上下文）

**ContextPackage结构**:
```json
{
  "chapter": 5,
  "selected_context": [
    {
      "source": "story_frame.md",
      "reason": "本章涉及主线冲突推进",
      "excerpt": "主角与反派的终极对决..."
    },
    {
      "source": "roles/主角.md",
      "reason": "主角性格一致性检查",
      "excerpt": "性格特征：坚韧、善良但有时冲动..."
    }
  ],
  "rule_stack": {
    "layers": [
      {
        "name": "hard_rules",
        "rules": ["主角不能死亡", "时间线不能跳跃"]
      },
      {
        "name": "soft_rules",
        "rules": ["对话比例30-40%", "每段不超过200字"]
      },
      {
        "name": "genre_rules",
        "rules": ["玄幻小说力量体系不能崩", "境界提升需要铺垫"]
      }
    ]
  }
}
```

#### Stage 3: Write（写作）

| 属性 | 值 |
|------|-----|
| **Agent** | Writer |
| **输入** | ChapterIntent + ContextPackage + 上章结尾片段 |
| **输出** | `ChapterDraft`（title + content + word_count） |
| **LLM温度** | 0.8（高温度，创造性写作） |
| **Token预算** | 8192（最大） |
| **验证** | 字数范围检查，标题格式检查 |

**输出格式**:
```
=== PRE_WRITE_CHECK ===
<写前检查清单>

=== CHAPTER_TITLE ===
第五章 面具之下

=== CHAPTER_CONTENT ===
<完整章节正文>
```

#### Stage 4: Settle（结算）

| 属性 | 值 |
|------|-----|
| **Agent** | Observer + Reflector |
| **输入** | 章节正文 + 当前StoryState |
| **输出** | `RuntimeStateDelta`（hook_ops + facts_new + summary_new） |
| **LLM温度** | 0.1（最低温度，精确提取） |
| **Token预算** | 4096 |
| **验证** | Delta格式校验，状态一致性检查 |

**核心职责**：
- **Observer**: 从正文中提取结构化事实（9类：角色/地点/资源/关系/情感/信息/钩子/时间/物理）
- **Reflector**: 将提取的事实更新到运行时状态

**RuntimeStateDelta结构**:
```json
{
  "chapter": 5,
  "hook_ops": [
    {"op": "upsert", "name": "神秘信件", "type": "foreshadowing", "status": "progressing", "description": "信件暗示更大阴谋"},
    {"op": "resolve", "name": "面具身份"}
  ],
  "facts_new": [
    {"subject": "李明", "predicate": "发现", "object": "师父的真实身份", "category": "information"},
    {"subject": "反派", "predicate": "是", "object": "师父的师兄", "category": "relationship"}
  ],
  "summary_new": {
    "chapter": 5,
    "title": "面具之下",
    "characters": ["李明", "师父", "反派"],
    "events": ["面具掉落", "身份揭露", "师徒对峙"],
    "state_changes": ["反派身份确认", "师徒信任破裂"],
    "mood": "紧张、震惊"
  }
}
```

#### Stage 5: Audit（审计）

| 属性 | 值 |
|------|-----|
| **Agent** | Auditor |
| **输入** | 章节正文 + StoryState + GenreProfile |
| **输出** | `AuditResult`（passed, score, issues[], summary） |
| **LLM温度** | 0.2（低温度，严格审查） |
| **Token预算** | 4096 |
| **验证** | 37维度评分，问题分类 |

**37维度审计**（参考InkOS）:

| 维度类别 | 具体维度 |
|---------|---------|
| **角色** | OOC（Out of Character）、性格一致性、行为合理性、对话风格 |
| **时间** | 时间线连续性、时间跨度合理性、因果关系 |
| **世界观** | 力量体系一致性、地理连续性、文化一致性 |
| **钩子** | 钩子推进、钩子遗忘、钩子兑现时机 |
| **叙事** | 节奏控制、悬念设置、信息密度、情感弧线 |
| **风格** | 文风一致性、视角一致性、语言风格 |
| **技术** | 字数范围、段落长度、对话比例 |
| **敏感内容** | 敏感词、版权风险 |

**AuditIssue结构**:
```json
{
  "severity": "critical",
  "category": "ooc_violation",
  "description": "主角在第3段表现出与已建立性格不符的懦弱行为",
  "suggestion": "将'他退缩了'改为'他强压怒火，冷静分析局势'"
}
```

#### Stage 6: Revise（修订）

| 属性 | 值 |
|------|-----|
| **Agent** | Reviser |
| **输入** | 章节正文 + AuditResult |
| **输出** | 修订后的章节正文 |
| **LLM温度** | 0.5（中等温度，平衡创造性与一致性） |
| **Token预算** | 8192 |
| **验证** | 重新审计通过检查 |

**6种修订模式**（参考InkOS）:
1. **spot-fix**: 精准修复（XML标记定位修改点）
2. **polish**: 表面润色（仅修改表达，不改内容）
3. **rewrite**: 段落重写（结构重组）
4. **rework**: 场景重构（大幅调整）
5. **auto**: 自动选择模式（根据问题严重程度）
6. **anti-detect**: 去AI化（打破句式模式，口语化替换）

**修订循环**:
```
Audit → 是否通过？
  ├── Yes → Persist
  └── No → 有 Critical 问题？
        ├── Yes → Revise → 重新 Audit → 循环（最多 max_revision_rounds 次）
        └── No → 记录 Warning，继续 Persist
```

#### Stage 7: Persist（持久化）

| 属性 | 值 |
|------|-----|
| **操作** | 原子写入章节 + 状态快照 + 索引同步 |
| **验证** | 真相文件一致性检查 |

**持久化内容**:
1. 章节文件：`books/<id>/chapters/chapter_0005.md`
2. 状态快照：`books/<id>/story/snapshots/chapter_0005.json`
3. 意图产物：`books/<id>/story/state/chapter_0005_intent.json`
4. 上下文产物：`books/<id>/story/state/chapter_0005_context.json`
5. 数据库记录：novels表更新word_count/chapter_count

### 2.3 流水线状态机

```
                    ┌──────────────────────────────────────┐
                    │                                      │
                    ▼                                      │
              ┌──────────┐                                 │
              │  Idle    │                                 │
              └────┬─────┘                                 │
                   │ write_next_chapter()                  │
                   ▼                                       │
              ┌──────────┐                                 │
              │ Planning │                                 │
              └────┬─────┘                                 │
                   │ plan complete                         │
                   ▼                                       │
              ┌──────────┐                                 │
              │Composing │                                 │
              └────┬─────┘                                 │
                   │ compose complete                      │
                   ▼                                       │
              ┌──────────┐                                 │
              │ Writing  │                                 │
              └────┬─────┘                                 │
                   │ write complete                        │
                   ▼                                       │
              ┌──────────┐                                 │
              │ Settling │                                 │
              └────┬─────┘                                 │
                   │ settle complete                       │
                   ▼                                       │
              ┌──────────┐     ┌──────────┐                │
              │ Auditing │────►│ Revising │────────────────┘
              └────┬─────┘     └────┬─────┘  (if critical issues)
                   │                │
                   │ passed         │ revised
                   ▼                │
              ┌──────────┐          │
              │Persisting│◄─────────┘
              └────┬─────┘
                   │
                   ▼
              ┌──────────┐
              │Completed │
              └──────────┘
```

---

## 3. Agent工程设计

### 3.1 Agent架构（双层模型）

参考 InkOS 的双层Agent架构：

```
┌─────────────────────────────────────────────────────────┐
│                    Agent System                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Layer 1: Pipeline Agents (Stateless, per-call)         │
│  ┌───────────────────────────────────────────────────┐  │
│  │  BaseAgent trait                                  │  │
│  │  ├── ArchitectAgent (建书)                        │  │
│  │  ├── PlannerAgent (规划)                          │  │
│  │  ├── ComposerAgent (编排)                          │  │
│  │  ├── WriterAgent (写作)                           │  │
│  │  ├── NormalizerAgent (字数标准化)                  │  │
│  │  ├── AuditorAgent (审计)                          │  │
│  │  ├── ReviserAgent (修订)                          │  │
│  │  ├── ObserverAgent (观察)                         │  │
│  │  └── ReflectorAgent (反射)                        │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  Layer 2: Interactive Agent (Stateful, conversation)    │
│  ┌───────────────────────────────────────────────────┐  │
│  │  ConversationAgent                                │  │
│  │  ├── Session management                           │  │
│  │  ├── Tool dispatch                                │  │
│  │  ├── Memory integration                           │  │
│  │  └── Context injection                            │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 3.2 BaseAgent Trait（已实现）

```rust
// src-tauri/src/domain/agents/base.rs

#[async_trait]
pub trait BaseAgent: Send + Sync {
    fn role(&self) -> AgentRole;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(
        &self,
        ctx: &AgentContext,
        input: AgentInput,
    ) -> Result<AgentOutput, AppError>;
}
```

**扩展方向**（参考Codex的Extension系统）：

```rust
// 新增生命周期hook
#[async_trait]
pub trait BaseAgent: Send + Sync {
    // 现有方法
    fn role(&self) -> AgentRole;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, ctx: &AgentContext, input: AgentInput) -> Result<AgentOutput, AppError>;

    // 新增hook点（可选实现，有默认空实现）
    async fn on_turn_start(&self, _ctx: &AgentContext) -> Result<(), AppError> { Ok(()) }
    async fn on_turn_end(&self, _ctx: &AgentContext, _output: &AgentOutput) -> Result<(), AppError> { Ok(()) }
    async fn on_error(&self, _ctx: &AgentContext, _error: &AppError) -> Result<(), AppError> { Ok(()) }
    fn build_system_prompt(&self, _ctx: &AgentContext) -> String { String::new() }
    fn validate_output(&self, _output: &AgentOutput) -> Result<(), AppError> { Ok(()) }
}
```

### 3.3 Agent配置系统

参考 Hermes 的声明式配置 + InkOS 的JSON驱动：

**配置目录**: `agents/*.json`

```json
// agents/_base.json（共享基础配置）
{
  "version": "1.0",
  "defaults": {
    "temperature": 0.7,
    "max_tokens": 4096,
    "timeout_secs": 120,
    "retry_count": 2,
    "output_format": "json"
  },
  "constraints": {
    "max_turns_per_session": 20,
    "max_tool_calls_per_turn": 5,
    "forbidden_patterns": ["暴力内容", "敏感政治"]
  }
}

// agents/writer.json（写手专用配置）
{
  "role": "writer",
  "name": "写手 (Writer)",
  "model": "gpt-4",
  "temperature": 0.8,
  "max_tokens": 8192,
  "prompt_template": "writer_system_prompt.md",
  "tools": ["read_file", "chapter_read", "novel_info"],
  "context": {
    "required": ["chapter_intent", "context_package", "previous_chapter"],
    "optional": ["genre_profile", "book_rules"],
    "token_budget": 12000
  },
  "output": {
    "format": "structured",
    "schema": "chapter_draft_schema.json",
    "validation": ["word_count_range", "title_format"]
  },
  "constraints": {
    "must_do": ["follow_chapter_intent", "respect_rule_stack"],
    "must_not_do": ["introduce_new_characters", "change_timeline"],
    "style_rules": ["对话比例30-40%", "每段不超过200字"]
  }
}
```

### 3.4 Agent角色定义

| Agent | 角色 | 温度 | Token | 输入 | 输出 |
|-------|------|------|-------|------|------|
| **Architect** | 建书师 | 0.7 | 4096 | 书籍brief | 书级框架（story_frame, volume_map, roles, rules） |
| **Planner** | 规划师 | 0.3 | 2048 | StoryState | ChapterIntent |
| **Composer** | 编排师 | 0.2 | 4096 | Intent + State + Rules | ContextPackage + RuleStack |
| **Writer** | 写手 | 0.8 | 8192 | Intent + Context | ChapterDraft |
| **Normalizer** | 标准化器 | 0.3 | 2048 | Draft + LengthSpec | NormalizedDraft |
| **Auditor** | 审计员 | 0.2 | 4096 | Draft + State + Genre | AuditResult |
| **Reviser** | 修订者 | 0.5 | 8192 | Draft + AuditResult | RevisedDraft |
| **Observer** | 观察者 | 0.1 | 4096 | ChapterContent | RuntimeStateDelta (facts) |
| **Reflector** | 反射器 | 0.2 | 4096 | Delta + State | UpdatedState |

---

## 4. 应用数据目录结构

### 4.1 全局数据目录

```
%APPDATA%/com.admin.mnemosyne/       (Windows)
~/Library/Application Support/com.admin.mnemosyne/  (macOS)
~/.local/share/com.admin.mnemosyne/  (Linux)
├── config.json                      # 全局应用配置
├── harness.json                     # Harness流水线配置
├── agents/                          # Agent配置
│   ├── _base.json                   # 共享基础配置
│   ├── architect.json
│   ├── planner.json
│   ├── composer.json
│   ├── writer.json
│   ├── normalizer.json
│   ├── auditor.json
│   ├── reviser.json
│   ├── observer.json
│   └── reflector.json
├── data/
│   ├── state.sqlite                 # 主数据库（novels, chapters, sessions, messages, agents）
│   ├── feedback.sqlite              # 反馈数据库（error_events, lessons, gate_evaluations）
│   └── logs.sqlite                  # 结构化日志
├── logs/                            # 滚动日志文件
│   └── mnemosyne.log.YYYY-MM-DD
├── skills/                          # 本地技能定义
│   ├── writing/
│   ├── editing/
│   └── publishing/
└── genres/                          # 类型配置
    ├── xianxia.yaml                 # 玄幻
    ├── urban.yaml                   # 都市
    ├── scifi.yaml                   # 科幻
    └── romance.yaml                 # 言情
```

### 4.2 项目数据目录（每个workspace）

```
workspace_root/
├── workspace.json                   # 工作区配置
├── novels/                          # 小说目录
│   └── <novel_id>/                  # 每本小说一个目录
│       ├── book.json                # 书籍配置（title, genre, platform, status）
│       ├── chapters/                # 章节文件
│       │   ├── chapter_0001.md
│       │   ├── chapter_0002.md
│       │   └── ...
│       ├── story/                   # 故事真相文件
│       │   ├── outline/
│       │   │   ├── story_frame.md   # 故事框架（4段：主题、冲突、世界、结局）
│       │   │   └── volume_map.md    # 卷册地图（5段 + 节奏原则）
│       │   ├── roles/               # 角色档案
│       │   │   ├── 主角/
│       │   │   │   └── 李明.md
│       │   │   ├── 反派/
│       │   │   │   └── 黑暗法师.md
│       │   │   └── 配角/
│       │   │       └── 老张.md
│       │   ├── runtime/             # 运行时状态
│       │   │   ├── state.json       # 结构化状态（StoryState）
│       │   │   ├── current_state.md # 人类可读状态投影
│       │   │   ├── hooks.md         # 钩子台账
│       │   │   ├── summaries.md     # 章节摘要
│       │   │   ├── subplots.md      # 支线追踪
│       │   │   ├── emotional_arcs.md # 情感弧线
│       │   │   └── character_matrix.md # 角色关系矩阵
│       │   ├── state/               # 流水线产物
│       │   │   ├── chapter_0001_intent.json
│       │   │   ├── chapter_0001_context.json
│       │   │   ├── chapter_0001_trace.json
│       │   │   └── ...
│       │   ├── snapshots/           # 状态快照（用于回滚）
│       │   │   ├── chapter_0001.json
│       │   │   ├── chapter_0002.json
│       │   │   └── ...
│       │   ├── book_rules.md        # 书级规则（禁令、风格、节奏）
│       │   ├── author_intent.md     # 作者长期创作方向
│       │   ├── current_focus.md     # 当前1-3章优先级
│       │   └── style_guide.md       # 写作方法论 + 风格指纹
│       └── config/                  # 书籍级harness配置
│           └── novel_harness.json   # 覆盖全局harness的书籍级配置
└── config/                          # 工作区配置
    └── workspace.json
```

### 4.3 路径管理（DataDir）

所有路径必须通过 `DataDir` getter 获取，禁止手动构建路径：

```rust
// src-tauri/src/infra/data_dir.rs

pub struct DataDir {
    base: PathBuf,  // %APPDATA%/com.admin.mnemosyne/
}

impl DataDir {
    // 全局配置
    pub fn config_file(&self) -> PathBuf { self.base.join("config.json") }
    pub fn harness_file(&self) -> PathBuf { self.base.join("harness.json") }
    pub fn agents_dir(&self) -> PathBuf { self.base.join("agents") }
    pub fn agent_config(&self, role: &str) -> PathBuf { self.agents_dir().join(format!("{}.json", role)) }

    // 数据库
    pub fn state_db(&self) -> PathBuf { self.base.join("data/state.sqlite") }
    pub fn feedback_db(&self) -> PathBuf { self.base.join("data/feedback.sqlite") }
    pub fn logs_db(&self) -> PathBuf { self.base.join("data/logs.sqlite") }

    // 日志
    pub fn log_dir(&self) -> PathBuf { self.base.join("logs") }

    // 类型配置
    pub fn genres_dir(&self) -> PathBuf { self.base.join("genres") }
    pub fn genre_profile(&self, genre: &str) -> PathBuf { self.genres_dir().join(format!("{}.yaml", genre)) }

    // 技能
    pub fn skills_dir(&self) -> PathBuf { self.base.join("skills") }
}
```

---

## 5. 数据库设计

### 5.1 数据库分离策略

参考 Hermes 的多库分离 + Codex的4库设计：

| 数据库 | 文件 | 职责 | 特点 |
|--------|------|------|------|
| **State DB** | `state.sqlite` | 核心业务数据 | 高频读写，WAL模式 |
| **Feedback DB** | `feedback.sqlite` | 反馈循环数据 | 追加写入，定期清理 |
| **Logs DB** | `logs.sqlite` | 结构化日志 | 只追加，定期归档 |

### 5.2 State DB Schema

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- ═══════════════════════════════════════════════════════════
-- 工作区
-- ═══════════════════════════════════════════════════════════
CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- ═══════════════════════════════════════════════════════════
-- 小说
-- ═══════════════════════════════════════════════════════════
CREATE TABLE novels (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    genre TEXT NOT NULL DEFAULT 'general',
    platform TEXT NOT NULL DEFAULT 'local',
    status TEXT NOT NULL DEFAULT 'drafting',
    language TEXT NOT NULL DEFAULT 'zh',
    word_count INTEGER NOT NULL DEFAULT 0,
    chapter_count INTEGER NOT NULL DEFAULT 0,
    target_chapters INTEGER NOT NULL DEFAULT 100,
    chapter_words INTEGER NOT NULL DEFAULT 3000,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

-- ═══════════════════════════════════════════════════════════
-- 章节元数据
-- ═══════════════════════════════════════════════════════════
CREATE TABLE chapters (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    number INTEGER NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'drafting',
    word_count INTEGER NOT NULL DEFAULT 0,
    audit_score REAL,
    revision_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
    UNIQUE(novel_id, number)
);

CREATE INDEX idx_chapters_novel ON chapters(novel_id, number);

-- ═══════════════════════════════════════════════════════════
-- Agent会话
-- ═══════════════════════════════════════════════════════════
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    novel_id TEXT,
    session_type TEXT NOT NULL DEFAULT 'chat',
    title TEXT NOT NULL DEFAULT '',
    summary TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cost REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE SET NULL
);

CREATE INDEX idx_sessions_novel ON sessions(novel_id);
CREATE INDEX idx_sessions_updated ON sessions(updated_at DESC);

-- ═══════════════════════════════════════════════════════════
-- 消息
-- ═══════════════════════════════════════════════════════════
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    tool_calls TEXT,
    tool_results TEXT,
    token_count INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_messages_session ON messages(session_id, created_at);

-- ═══════════════════════════════════════════════════════════
-- Agent配置
-- ═══════════════════════════════════════════════════════════
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    model TEXT NOT NULL DEFAULT 'gpt-4',
    system_prompt TEXT NOT NULL DEFAULT '',
    temperature REAL NOT NULL DEFAULT 0.7,
    max_tokens INTEGER NOT NULL DEFAULT 4096,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL
);

-- ═══════════════════════════════════════════════════════════
-- 提示词管理
-- ═══════════════════════════════════════════════════════════
CREATE TABLE prompts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'general',
    tags TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_prompts_category ON prompts(category);

-- ═══════════════════════════════════════════════════════════
-- 趋势扫描
-- ═══════════════════════════════════════════════════════════
CREATE TABLE trends (
    id TEXT PRIMARY KEY,
    keyword TEXT NOT NULL,
    platform TEXT NOT NULL,
    score REAL NOT NULL DEFAULT 0.0,
    metadata TEXT NOT NULL DEFAULT '{}',
    scanned_at TEXT NOT NULL
);

CREATE INDEX idx_trends_keyword ON trends(keyword);
CREATE INDEX idx_trends_platform ON trends(platform);
```

### 5.3 Feedback DB Schema

```sql
PRAGMA journal_mode = WAL;

-- ═══════════════════════════════════════════════════════════
-- 错误事件
-- ═══════════════════════════════════════════════════════════
CREATE TABLE error_events (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    chapter_number INTEGER NOT NULL,
    agent_role TEXT NOT NULL,
    error_type TEXT NOT NULL,
    dimension TEXT,
    severity TEXT NOT NULL DEFAULT 'warning',
    description TEXT NOT NULL,
    suggestion TEXT,
    lesson_id TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_error_events_novel ON error_events(novel_id, error_type);
CREATE INDEX idx_error_events_lesson ON error_events(lesson_id);

-- ═══════════════════════════════════════════════════════════
-- 约束教训
-- ═══════════════════════════════════════════════════════════
CREATE TABLE lessons (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    error_type TEXT NOT NULL,
    constraint_text TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL DEFAULT 0,
    first_seen_chapter INTEGER NOT NULL,
    last_seen_chapter INTEGER NOT NULL,
    state TEXT NOT NULL DEFAULT 'active',
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    suppressed_at TEXT,
    archived_at TEXT
);

CREATE INDEX idx_lessons_novel_state ON lessons(novel_id, state);
CREATE INDEX idx_lessons_novel_type ON lessons(novel_id, error_type);

-- ═══════════════════════════════════════════════════════════
-- 质量门评估
-- ═══════════════════════════════════════════════════════════
CREATE TABLE gate_evaluations (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    chapter_number INTEGER NOT NULL,
    stage TEXT NOT NULL,
    total_gates INTEGER NOT NULL,
    passed_gates INTEGER NOT NULL,
    failed_gates INTEGER NOT NULL,
    overall_passed INTEGER NOT NULL,
    recommended_action TEXT NOT NULL,
    evaluation_time_ms INTEGER NOT NULL,
    gate_results TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_gate_eval_novel ON gate_evaluations(novel_id, chapter_number);
CREATE INDEX idx_gate_eval_stage ON gate_evaluations(novel_id, stage);

-- ═══════════════════════════════════════════════════════════
-- 流水线运行记录
-- ═══════════════════════════════════════════════════════════
CREATE TABLE pipeline_runs (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    chapter_number INTEGER NOT NULL,
    stage TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    started_at TEXT NOT NULL,
    completed_at TEXT,
    duration_ms INTEGER,
    tokens_used INTEGER,
    cost REAL,
    error_message TEXT,
    metadata TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_pipeline_runs_novel ON pipeline_runs(novel_id, chapter_number);
CREATE INDEX idx_pipeline_runs_stage ON pipeline_runs(novel_id, stage);
```

### 5.4 Logs DB Schema

```sql
PRAGMA journal_mode = WAL;

-- ═══════════════════════════════════════════════════════════
-- 结构化日志
-- ═══════════════════════════════════════════════════════════
CREATE TABLE log_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    level TEXT NOT NULL,
    module TEXT NOT NULL,
    message TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    session_id TEXT,
    novel_id TEXT
);

CREATE INDEX idx_log_entries_timestamp ON log_entries(timestamp DESC);
CREATE INDEX idx_log_entries_level ON log_entries(level);
CREATE INDEX idx_log_entries_session ON log_entries(session_id);
```

---

## 6. 记忆系统设计

### 6.1 记忆层次模型

参考 Hermes 的 MemoryProvider + InkOS 的 truth files：

```
┌─────────────────────────────────────────────────────────┐
│                    Memory System                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  L1: Working Memory (会话内)                            │
│  ┌───────────────────────────────────────────────────┐  │
│  │  当前对话历史 + 工具调用结果                        │  │
│  │  生命周期：单次会话                                 │  │
│  │  存储：内存                                        │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  L2: Chapter Memory (章节级)                            │
│  ┌───────────────────────────────────────────────────┐  │
│  │  RuntimeStateDelta + ChapterSummary               │  │
│  │  生命周期：永久                                    │  │
│  │  存储：SQLite + 文件                               │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  L3: Novel Memory (书籍级)                              │
│  ┌───────────────────────────────────────────────────┐  │
│  │  StoryState + Truth Files + Snapshots             │  │
│  │  生命周期：永久                                    │  │
│  │  存储：文件系统 + SQLite                           │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  L4: Global Memory (跨书籍)                             │
│  ┌───────────────────────────────────────────────────┐  │
│  │  ConstraintLessons + Skills + GenreProfiles       │  │
│  │  生命周期：永久                                    │  │
│  │  存储：SQLite + 文件                               │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 6.2 记忆读写流程

```
写入流程（每章完成后）:
  ChapterContent
    │
    ▼
  ObserverAgent → RuntimeStateDelta
    │
    ▼
  ReflectorAgent → apply_delta() → StoryState更新
    │
    ▼
  StateManager.save_state() → state.json
    │
    ▼
  StateManager.save_snapshot() → snapshots/chapter_NNNN.json
    │
    ▼
  Database (chapters表更新 word_count, status)

读取流程（每章开始前）:
  Chapter Number
    │
    ▼
  StateManager.load_state() → StoryState
    │
    ├──► hooks (钩子台账)
    ├──► facts (事实三元组)
    ├──► summaries (章节摘要)
    │
    ▼
  ContextBuilder.assemble() → 完整上下文
    │
    ▼
  Agent.execute() (注入上下文)
```

### 6.3 事实三元组（Temporal Facts）

参考 InkOS 的事实系统：

```rust
pub struct TemporalFact {
    pub fact_id: String,
    pub subject: String,      // 主语（角色、地点、物品）
    pub predicate: String,    // 谓语（动作、状态、关系）
    pub object: String,       // 宾语
    pub category: FactCategory,
    pub valid_from_chapter: u32,
    pub valid_until_chapter: Option<u32>,  // None = 仍然有效
    pub source_chapter: u32,
    pub created_at: String,
}

pub enum FactCategory {
    Character,     // 角色相关
    Location,      // 地点相关
    Resource,      // 资源相关
    Relationship,  // 关系相关
    Emotion,       // 情感相关
    Information,   // 信息相关
    Hook,          // 钩子相关
    Time,          // 时间相关
    Physical,      // 物理相关
}
```

### 6.4 钩子生命周期

```
Hook状态机:
  Open ──► Progressing ──► Resolved
    │            │
    │            ▼
    │         Deferred ──► Open (重新激活)
    │
    ▼
  Abandoned (超时未推进，由审计员检测)

钩子台账:
  ┌────────────┬──────────┬──────────┬──────────┬──────────┐
  │    Name    │  Type    │  Status  │  Start   │  Last    │
  ├────────────┼──────────┼──────────┼──────────┼──────────┤
  │ 神秘信件   │ foreshadow│ open     │ ch.3     │ ch.4     │
  │ 师徒矛盾   │ conflict │ progress │ ch.1     │ ch.5     │
  │ 身世之谜   │ mystery  │ deferred │ ch.2     │ ch.4     │
  └────────────┴──────────┴──────────┴──────────┴──────────┘
```

---

## 7. 上下文管理设计

### 7.1 上下文组装流程

参考 InkOS 的 Input Governance + Hermes 的三层提示：

```
┌─────────────────────────────────────────────────────────┐
│                 Context Assembly Pipeline                 │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Step 1: 加载真相文件 (从磁盘)                           │
│  ┌───────────────────────────────────────────────────┐  │
│  │  story_frame.md → volume_map.md → roles/*.md     │  │
│  │  → book_rules.md → author_intent.md              │  │
│  │  → current_focus.md → style_guide.md             │  │
│  └───────────────────────────────────────────────────┘  │
│                         │                               │
│                         ▼                               │
│  Step 2: 压缩大文件 (>6000字符 → 标题索引)              │
│  ┌───────────────────────────────────────────────────┐  │
│  │  每个文件提取标题，最多80个标题                     │  │
│  └───────────────────────────────────────────────────┘  │
│                         │                               │
│                         ▼                               │
│  Step 3: 加载运行时状态                                 │
│  ┌───────────────────────────────────────────────────┐  │
│  │  StoryState (hooks, facts, summaries)            │  │
│  │  + 章节摘要 (最近3章)                             │  │
│  │  + 活跃钩子列表                                   │  │
│  └───────────────────────────────────────────────────┘  │
│                         │                               │
│                         ▼                               │
│  Step 4: 加载约束教训 (从Feedback DB)                   │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Active ConstraintLessons for this novel          │  │
│  └───────────────────────────────────────────────────┘  │
│                         │                               │
│                         ▼                               │
│  Step 5: 组装最终上下文                                 │
│  ┌───────────────────────────────────────────────────┐  │
│  │  System Prompt (Agent专用)                        │  │
│  │  + Truth Files (压缩后)                           │  │
│  │  + Runtime State                                  │  │
│  │  + Constraint Lessons                             │  │
│  │  + Chapter Intent (如果是写手)                     │  │
│  │  + Context Package (如果是写手)                    │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 7.2 上下文预算管理

参考 Codex 的 Context Fragment + Hermes 的 token budget：

```rust
pub struct ContextBudget {
    pub max_system_prompt_tokens: u32,    // 默认 4096
    pub max_context_window_tokens: u32,   // 默认 32768
    pub max_user_message_tokens: u32,     // 默认 16384
    pub reserved_for_response: u32,       // 默认 8192
}

pub struct ContextFragment {
    pub name: String,
    pub priority: u32,           // 1-10, 越高越优先
    pub token_estimate: u32,
    pub content: String,
    pub compressible: bool,      // 是否可以压缩
    pub required: bool,          // 是否必须包含
}
```

**优先级排序**:
1. Agent系统提示（必须，不可压缩）
2. 章节意图（必须，不可压缩）
3. 上下文包（必须，可压缩）
4. 角色档案（高优先级，可压缩）
5. 运行时状态（高优先级，可压缩）
6. 章节摘要（中优先级，可压缩）
7. 真相文件（低优先级，可压缩）
8. 约束教训（低优先级，可压缩）

### 7.3 上下文压缩策略

```
当上下文超出token预算时:

1. 首先压缩低优先级可压缩片段
   - 真相文件 > 6000字符 → 标题索引
   - 章节摘要 > 3章 → 只保留最近3章

2. 然后压缩中优先级片段
   - 运行时状态 → 只保留活跃钩子 + 最近事实

3. 最后压缩高优先级片段
   - 角色档案 → 只保留当前章出场角色

4. 绝不压缩
   - Agent系统提示
   - 章节意图
   - 上下文包
```

---

## 8. 工具系统设计

### 8.1 工具架构

参考 Hermes 的自动发现 + Codex的ToolOrchestrator：

```
┌─────────────────────────────────────────────────────────┐
│                    Tool System                           │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │  ToolRegistry (HashMap<String, Box<dyn ToolExec>>)│  │
│  │  ├── ReadFileTool                                │  │
│  │  ├── WriteFileTool                               │  │
│  │  ├── ListDirTool                                 │  │
│  │  ├── GrepTool                                    │  │
│  │  ├── GlobTool                                    │  │
│  │  ├── NovelInfoTool                               │  │
│  │  ├── ChapterReadTool                             │  │
│  │  ├── ChapterListTool                             │  │
│  │  ├── NovelListTool                               │  │
│  │  ├── SearchTool (FTS5)                           │  │
│  │  └── WebSearchTool (可选)                         │  │
│  └───────────────────────────────────────────────────┘  │
│                         │                               │
│                         ▼                               │
│  ┌───────────────────────────────────────────────────┐  │
│  │  ToolOrchestrator                                │  │
│  │  ├── Permission Check (Agent允许使用哪些工具?)     │  │
│  │  ├── Rate Limit Check (调用频率限制)               │  │
│  │  ├── Sandbox Check (沙箱策略)                     │  │
│  │  ├── Approval Check (是否需要用户确认?)           │  │
│  │  └── Execute (实际执行)                           │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 8.2 工具权限模型

```rust
pub struct ToolPermission {
    pub allowed_agents: Option<Vec<String>>,  // 允许使用的Agent角色
    pub max_calls_per_turn: Option<u32>,      // 每轮最大调用次数
    pub requires_novel_context: bool,         // 是否需要小说上下文
    pub requires_approval: bool,              // 是否需要用户确认
}

// 工具权限配置
pub fn tool_permissions() -> HashMap<String, ToolPermission> {
    let mut m = HashMap::new();
    m.insert("read_file".into(), ToolPermission {
        allowed_agents: None,  // 所有Agent可用
        max_calls_per_turn: Some(10),
        requires_novel_context: false,
        requires_approval: false,
    });
    m.insert("write_file".into(), ToolPermission {
        allowed_agents: Some(vec!["writer".into(), "reviser".into()]),
        max_calls_per_turn: Some(3),
        requires_novel_context: true,
        requires_approval: true,  // 写文件需要确认
    });
    m.insert("grep".into(), ToolPermission {
        allowed_agents: None,
        max_calls_per_turn: Some(5),
        requires_novel_context: false,
        requires_approval: false,
    });
    m
}
```

### 8.3 工具定义格式

参考 Hermes 的 schema 格式：

```rust
pub trait ToolExecutor: Send + Sync {
    fn spec(&self) -> ToolSpec;
    fn execute(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolOutput, AppError>;
    fn permission(&self) -> ToolPermission { ToolPermission::default() }
}

pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,  // JSON Schema
}
```

---

## 9. 配置系统设计

### 9.1 配置层次

参考 Codex 的7层配置 + InkOS 的4层配置：

```
配置优先级（从低到高）:
  1. 内置默认值 (代码硬编码)
  2. 全局配置 (config.json)
  3. Harness配置 (harness.json)
  4. Agent配置 (agents/*.json)
  5. 类型配置 (genres/*.yaml)
  6. 书籍级配置 (novel_harness.json)
  7. 运行时覆盖 (CLI参数/前端设置)
```

### 9.2 配置文件格式

```json
// config.json - 全局应用配置
{
  "version": "1.0",
  "ui": {
    "theme": "dark",
    "locale": "zh-CN",
    "log_level": "info"
  },
  "llm": {
    "default_provider": "openai",
    "default_model": "gpt-4",
    "providers": {
      "openai": {
        "api_key": "sk-...",
        "base_url": "https://api.openai.com/v1"
      },
      "ollama": {
        "base_url": "http://localhost:11434"
      }
    }
  },
  "agent": {
    "max_concurrent_runs": 1,
    "default_timeout_secs": 120
  }
}
```

```json
// harness.json - 流水线配置
{
  "version": "1.0",
  "pipeline": {
    "stage_order": ["plan", "compose", "write", "settle", "audit", "revise"],
    "required_stages": ["plan", "compose", "write", "settle", "audit"],
    "max_revision_rounds": 3,
    "audit_pass_threshold": 70.0,
    "improvement_epsilon": 0.01
  },
  "quality_gates": [
    {
      "id": "word_count",
      "name": "字数范围",
      "stage": "audit",
      "gate_type": "word_count_range",
      "threshold": 0.85,
      "action_on_fail": "revise"
    },
    {
      "id": "critical_issues",
      "name": "严重问题数",
      "stage": "audit",
      "gate_type": "issue_count",
      "threshold": 0,
      "action_on_fail": "revise"
    }
  ],
  "feedback_rules": [
    {
      "id": "ooc_lesson",
      "trigger": {
        "error_type": "ooc_violation",
        "min_occurrences": 3,
        "scope": "novel"
      },
      "constraint": "角色行为必须符合已建立的性格特征",
      "target": "all_agents",
      "cooldown_chapters": 5
    }
  ],
  "gc_policy": {
    "stale_snapshot_days": 90,
    "max_snapshots_per_novel": 50,
    "compact_state_every_n_chapters": 10,
    "archive_completed_novels": true
  }
}
```

---

## 10. 安全与沙箱设计

### 10.1 安全模型

参考 Codex 的多层沙箱 + Hermes 的终端后端：

```
┌─────────────────────────────────────────────────────────┐
│                    Security Layers                       │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Layer 1: IPC边界验证 (Tauri Capabilities)              │
│  ┌───────────────────────────────────────────────────┐  │
│  │  前端代码不可信，所有输入必须验证                    │  │
│  │  只有声明的command可调用                            │  │
│  │  权限粒度：操作级 > 资源级                          │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  Layer 2: 工具权限控制                                  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  每个工具声明允许的Agent角色                        │  │
│  │  每个工具声明调用频率限制                           │  │
│  │  危险操作需要用户确认                               │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  Layer 3: 路径安全                                     │
│  ┌───────────────────────────────────────────────────┐  │
│  │  禁止 ../ 路径穿越                                 │  │
│  │  所有文件操作限制在workspace内                      │  │
│  │  DataDir getters 保护全局数据                      │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  Layer 4: 输入验证                                     │
│  ┌───────────────────────────────────────────────────┐  │
│  │  所有#[command]函数验证输入类型/长度/格式           │  │
│  │  LLM输出JSON Schema校验                           │  │
│  │  敏感词过滤（可选）                                 │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 10.2 沙箱执行环境

当前阶段（v0.1.0）：本地执行，无沙箱。
生产阶段：参考 Codex 的 OS原生沙箱。

```rust
// src-tauri/src/infra/sandbox/

pub enum SandboxType {
    None,           // 本地执行（开发阶段）
    Restricted,     // 受限执行（生产阶段）
}

pub struct SandboxEnforcer {
    sandbox_type: SandboxType,
    allowed_paths: Vec<PathBuf>,
    denied_paths: Vec<PathBuf>,
}

impl SandboxEnforcer {
    pub fn enforce(&self, command: &str, ctx: &ToolContext) -> Result<(), AppError> {
        match self.sandbox_type {
            SandboxType::None => Ok(()),
            SandboxType::Restricted => {
                // 验证路径安全性
                // 验证命令是否在允许列表中
                // 验证是否需要用户确认
                Ok(())
            }
        }
    }
}
```

---

## 11. 前端架构设计

### 11.1 页面结构

```
src/pages/
├── DashboardPage.tsx          # 仪表盘（统计、最近活动）
├── NovelsPage.tsx             # 小说列表
├── NovelDetailPage.tsx        # 小说详情（章节列表、状态）
├── ChapterEditorPage.tsx      # 章节编辑器
├── AgentChatPage.tsx          # Agent对话界面
├── PromptsPage.tsx            # 提示词管理
├── TrendsPage.tsx             # 趋势扫描
├── SettingsPage.tsx           # 设置
│   ├── LLMSettings.tsx        # LLM配置
│   ├── AgentSettings.tsx      # Agent配置
│   ├── HarnessSettings.tsx    # Harness配置
│   └── GeneralSettings.tsx    # 通用设置
└── RadarPage.tsx              # 市场雷达
```

### 11.2 状态管理

```
src/stores/
├── workspaceStore.ts          # Zustand: 工作区状态
├── novelStore.ts              # Zustand: 小说状态
├── pipelineStore.ts           # Zustand: 流水线状态
├── chatStore.ts               # Zustand: 对话状态
└── settingsStore.ts           # Zustand: 设置状态

src/lib/store.tsx              # useReducer + Context: 全局App状态
```

### 11.3 Hook层

```
src/hooks/
├── useNovels.ts               # 小说CRUD
├── useChapters.ts             # 章节CRUD
├── usePipeline.ts             # 流水线控制
├── useAgentChat.ts            # Agent对话
├── usePrompts.ts              # 提示词管理
├── useTrends.ts               # 趋势数据
├── useSettings.ts             # 设置管理
└── useStreaming.ts            # 流式响应
```

---

## 12. 状态管理设计

### 12.1 状态层次

```
┌─────────────────────────────────────────────────────────┐
│                    State Management                      │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  L1: UI State (React)                                  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  当前页面、模态框状态、加载状态、错误状态           │  │
│  │  存储：React state + Context                      │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  L2: Domain State (Zustand)                             │
│  ┌───────────────────────────────────────────────────┐  │
│  │  小说列表、章节列表、流水线状态、对话历史          │  │
│  │  存储：Zustand + localStorage                     │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  L3: Persistent State (SQLite + Files)                  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  StoryState, Truth Files, Snapshots               │  │
│  │  存储：SQLite + 文件系统                          │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  L4: Configuration State (JSON Files)                   │
│  ┌───────────────────────────────────────────────────┐  │
│  │  config.json, harness.json, agents/*.json         │  │
│  │  存储：JSON文件                                   │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 12.2 状态同步

```
前端操作
  │
  ▼
IPC Command (camelCase)
  │
  ▼
Rust Service (snake_case)
  │
  ├──► Database (SQLite)
  ├──► File System (State Files)
  └──► IPC Response
        │
        ▼
Frontend State Update (Zustand/Context)
```

---

## 13. 质量门与反馈循环

### 13.1 质量门类型

```rust
pub enum GateType {
    ScoreThreshold,      // 审计分数阈值
    IssueCount,          // 问题数量限制
    WordCountRange,      // 字数范围
    ForbiddenPattern,    // 禁止模式
    CompletenessCheck,   // 完整性检查
    DimensionScore,      // 单维度分数
    ConsistencyCheck,    // 一致性检查
    CustomRule,          // 自定义规则
}
```

### 13.2 反馈循环流程

```
审计发现错误
  │
  ▼
记录 error_event (Feedback DB)
  │
  ▼
检查是否触发 feedback_rule
  │
  ├── Yes → 生成 constraint_lesson
  │         │
  │         ▼
  │    注入到 Agent Prompt
  │         │
  │         ▼
  │    后续章节避免同类错误
  │
  └── No → 仅记录，不生成教训
```

### 13.3 垃圾回收

```
GC触发条件:
  - 每N章自动执行 (compact_state_every_n_chapters)
  - 手动触发

GC操作:
  1. 清理过期快照 (stale_snapshot_days)
  2. 去重事实 (deduplicate facts)
  3. 压缩摘要 (trim summaries)
  4. 归档已完成书籍 (archive_completed_novels)
```

---

## 14. 实施路线图

### Phase 1: 基础架构 (v0.2.0)

- [ ] 完善 `DataDir` 路径管理
- [ ] 重构数据库为3库分离 (state/feedback/logs)
- [ ] 实现 `BaseAgent` 生命周期hook
- [ ] 完善工具权限模型
- [ ] 实现配置层次合并

### Phase 2: 流水线引擎 (v0.3.0)

- [ ] 实现7阶段流水线状态机
- [ ] 实现 Input Governance (ContextPackage + RuleStack)
- [ ] 实现 State Settlement (Observer + Reflector)
- [ ] 实现质量门评估
- [ ] 实现反馈循环

### Phase 3: Agent系统 (v0.4.0)

- [ ] 实现9个专业Agent
- [ ] 实现Agent配置热加载
- [ ] 实现上下文预算管理
- [ ] 实现上下文压缩策略
- [ ] 实现约束教训注入

### Phase 4: 前端集成 (v0.5.0)

- [ ] 实现流水线控制面板
- [ ] 实现Agent对话界面
- [ ] 实现章节编辑器
- [ ] 实现实时状态同步
- [ ] 实现设置管理界面

### Phase 5: 生产化 (v1.0.0)

- [ ] 实现OS原生沙箱
- [ ] 实现OpenTelemetry集成
- [ ] 实现JSONL rollout持久化
- [ ] 实现MCP工具协议
- [ ] 性能优化与测试

---

## 附录A: 关键类型定义

```rust
// 所有核心类型已在 src-tauri/src/domain/agents/types.rs 中定义
// 主要类型包括:
// - AgentRole (9个角色)
// - AgentInput / AgentOutput
// - ChapterIntent / ContextPackage / RuntimeStateDelta
// - AuditResult / AuditIssue
// - StoryState / HookRecord / TemporalFact
// - BookConfig / ChapterMeta / GenreProfile
// - LengthSpec / LengthCheck
```

## 附录B: 参考项目

| 项目 | 参考内容 | 优先级 |
|------|---------|--------|
| **InkOS** | 7阶段流水线、Input Governance、State Settlement、双层Agent | P0 |
| **Hermes** | Tool自动发现、Footprint Ladder、可插拔ContextEngine、三层提示 | P1 |
| **Codex** | Extension系统、JSONL rollout、多层配置、MCP集成 | P2 |
