# Workflow Manifest（工作流清单）

> [English](workflow.md) · **中文**

**Workflow（工作流）** 制品定义了 **Agent 按步执行的编排逻辑约束**。
它是一个声明式的**状态机**：每个步骤告诉 Agent 要做什么动作、往共享 Context 里写什么、
以及满足什么条件才能跳转到下一步。Agent 是执行者，Workflow 是它必须遵循的脚本。

```
Workflow  =  入口步骤（entry）
           + 共享 Context（Agent 读写的类型化状态）
           + Steps（decision | skill | agent | parallel | loop）
           + Transitions（条件 → 下一步）
           + 终止状态（__end__）
```

## 最简示例

```toml
[package]
kind        = "workflow"
namespace   = "myorg"
name        = "quick-review"
description = "Agent 驱动的代码审查，单个 skill 步骤"

[workflow]
entry = "review"            # agent 执行的第一个步骤

[workflow.context]
pr_url = { type = "string", required = true }
score  = { type = "integer", default = 0 }

[[workflow.steps]]
id     = "review"
kind   = "skill"
use    = "agentverse-ci/code-reviewer@>=0.1.0"
inputs = { diff = "{{context.pr_url}}", rules = ["correctness"] }
writes = ["score"]

[[workflow.steps.transitions]]
goto = "__end__"            # 终止 — agent 在此停止

[metadata]
tags    = ["workflow", "code-review"]
license = "MIT"
```

## 完整示例 — PR 审查（分诊 + 分支 + 重试）

```toml
[package]
kind        = "workflow"
namespace   = "myorg"
name        = "pr-review-flow"
description = "Agent 驱动的 PR 审查：分诊 → 按深度分支 → 批准或请求修改"

# ── 工作流入口与全局设置 ──────────────────────────────────────────────────────
[workflow]
entry   = "triage"          # agent 必须执行的第一个步骤
timeout = "30m"

# ── 共享 Context 模式（Agent 读写的类型化状态）────────────────────────────────
[workflow.context]
pr_url   = { type = "string",  required = true,  description = "调用时传入的 PR URL" }
depth    = { type = "string",  default = "shallow", enum = ["shallow", "deep"] }
score    = { type = "integer", default = 0 }
issues   = { type = "array",   default = [] }
approved = { type = "boolean", default = false }

# ── 步骤 1：分诊 — Agent 决策审查深度 ────────────────────────────────────────
[[workflow.steps]]
id          = "triage"
name        = "分诊 PR 范围"
kind        = "decision"        # agent 推理并写入 context
instruction = """
获取 {{context.pr_url}} 处的 PR，统计总变更行数。
若变更超过 500 行或涉及安全敏感路径，将 depth 设为 "deep"。
否则设为 "shallow"。
"""
writes = ["depth"]              # 声明此步骤可修改的 context 键

[[workflow.steps.transitions]]
when = "context.depth == 'deep'"
goto = "full-review"

[[workflow.steps.transitions]]
when = "context.depth == 'shallow'"
goto = "quick-check"

# ── 步骤 2a：完整审查（deep 路径）────────────────────────────────────────────
[[workflow.steps]]
id       = "full-review"
name     = "完整代码审查"
kind     = "skill"
use      = "agentverse-ci/code-reviewer@>=0.1.0"
inputs   = { diff = "{{context.pr_url}}", rules = ["security", "correctness", "performance", "style"] }
writes   = ["score", "issues"]
on_error = "retry"
retry    = { max_attempts = 2, backoff = "30s" }

[[workflow.steps.transitions]]
when = "context.score >= 80"
goto = "approve"

[[workflow.steps.transitions]]
when = "context.score < 80"
goto = "request-changes"

# ── 步骤 2b：快速检查（shallow 路径）─────────────────────────────────────────
[[workflow.steps]]
id     = "quick-check"
name   = "仅做风格检查"
kind   = "skill"
use    = "agentverse-ci/code-reviewer@>=0.1.0"
inputs = { diff = "{{context.pr_url}}", rules = ["style"] }
writes = ["score", "issues"]

[[workflow.steps.transitions]]
goto = "approve"              # 无条件 — 总是前往 approve

# ── 步骤 3a：批准 ─────────────────────────────────────────────────────────────
[[workflow.steps]]
id     = "approve"
name   = "发布批准评论"
kind   = "skill"
use    = "agentverse-ci/pr-commenter@>=0.1.0"
inputs = { message = "✅ 审查通过（得分：{{context.score}}）", approve = true }
writes = ["approved"]

[[workflow.steps.transitions]]
goto = "__end__"

# ── 步骤 3b：请求修改 ─────────────────────────────────────────────────────────
[[workflow.steps]]
id     = "request-changes"
name   = "发布修改请求"
kind   = "skill"
use    = "agentverse-ci/pr-commenter@>=0.1.0"
inputs = { issues = "{{context.issues}}", score = "{{context.score}}", approve = false }

[[workflow.steps.transitions]]
goto = "__end__"

[metadata]
tags     = ["workflow", "agent", "orchestration", "state-machine", "branching"]
homepage = "https://github.com/myorg/workflows"
license  = "MIT"
```

## 字段参考

### `[workflow]`

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `entry` | string | ✅ | Agent 执行的第一个步骤 ID |
| `timeout` | string | — | 全局执行超时时间（如 `30m`、`2h`） |

### `[workflow.context]`

定义所有步骤共享的**类型化状态**。Agent 运行时在调用时初始化 context，并在每个步骤后更新它。

```toml
[workflow.context]
pr_url = { type = "string",  required = true }
score  = { type = "integer", default = 0 }
depth  = { type = "string",  default = "shallow", enum = ["shallow", "deep"] }
issues = { type = "array",   default = [] }
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `type` | string | `string`、`integer`、`boolean`、`array`、`object` |
| `required` | boolean | 为 true 时，调用时必须提供 |
| `default` | any | 未提供时的默认值 |
| `enum` | array | `string` 类型的允许值列表 |
| `description` | string | 文档注释 |

### `[[workflow.steps]]`

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | ✅ | 唯一步骤标识符；由 `transitions.goto` 引用 |
| `name` | string | — | 人类可读的步骤名称 |
| `kind` | string | ✅ | `decision`、`skill`、`agent`、`parallel`、`loop` |
| `use` | string | 条件 | `namespace/name@version`，`skill`/`agent` 类型必填 |
| `inputs` | object | — | 键值映射；支持 `{{context.*}}` 和 `{{env.*}}` |
| `writes` | string[] | — | 此步骤允许修改的 context 键 |
| `instruction` | string | 条件 | `decision` 类型的自然语言指令 |
| `on_error` | string | — | `fail`（默认）、`warn`、`continue`、`retry` |
| `retry` | object | — | `on_error = "retry"` 时的 `max_attempts` 和 `backoff` |
| `timeout` | string | — | 单步超时时间，覆盖工作流级别 |

### `[[workflow.steps.transitions]]`

Transitions 控制步骤完成后 Agent 去往哪里。**按顺序评估**，第一个匹配的条件获胜。

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `when` | string | — | 基于 `context.*` 的布尔表达式；省略则为无条件跳转 |
| `goto` | string | ✅ | 下一步骤的 ID，或 `__end__` 终止工作流 |

**表达式语法：** `context.score >= 80`、`context.depth == 'deep'`、`!context.approved`

## 步骤类型详解

### `decision` — Agent 推理并更新 context

Agent 接收 `instruction`，读取当前 context，将结果写回 `writes` 声明的键。不调用外部 skill。

```toml
[[workflow.steps]]
id          = "triage"
kind        = "decision"
instruction = "如果 context.pr_url 涉及超过 500 行，将 depth 设为 'deep'。"
writes      = ["depth"]

[[workflow.steps.transitions]]
when = "context.depth == 'deep'"
goto = "full-review"

[[workflow.steps.transitions]]
when = "context.depth == 'shallow'"
goto = "quick-check"
```

### `skill` — 调用 AgentVerse skill

```toml
[[workflow.steps]]
id     = "full-review"
kind   = "skill"
use    = "agentverse-ci/code-reviewer@>=0.1.0"
inputs = { diff = "{{context.pr_url}}", rules = ["security", "correctness"] }
writes = ["score", "issues"]

[[workflow.steps.transitions]]
when = "context.score >= 80"
goto = "approve"

[[workflow.steps.transitions]]
goto = "request-changes"   # 兜底 — 无 `when` = 始终匹配
```

### `parallel` — 同时展开到多个分支

```toml
[[workflow.steps]]
id       = "scan-all"
kind     = "parallel"
branches = ["security-scan", "lint-check", "type-check"]
join     = "all"           # all | any | first

[[workflow.steps.transitions]]
goto = "aggregate"
```

### `loop` — 重复执行直到满足条件

```toml
[[workflow.steps]]
id             = "fix-loop"
kind           = "loop"
body           = "run-tests"   # 要重复执行的步骤
until          = "context.tests_pass == true"
max_iterations = 5

[[workflow.steps.transitions]]
when = "loop.done"
goto = "deploy"

[[workflow.steps.transitions]]
when = "loop.failed"
goto = "notify-failure"
```

### `agent` — 委托给另一个 AgentVerse 代理

```toml
[[workflow.steps]]
id     = "security-agent"
kind   = "agent"
use    = "myorg/security-auditor@>=1.0.0"
inputs = { repo = "{{context.pr_url}}" }
writes = ["security_score"]

[[workflow.steps.transitions]]
goto = "decide-merge"
```

## Context 模板语法

在 `inputs`、`instruction` 和 `transitions.when` 中使用 `{{...}}` 引用值：

| 表达式 | 说明 |
|--------|------|
| `{{context.pr_url}}` | 读取 context 键 |
| `{{env.GITHUB_TOKEN}}` | 读取环境变量 |
| `context.score >= 80` | 数值比较（用于 `when`） |
| `context.depth == 'deep'` | 字符串相等（用于 `when`） |
| `!context.approved` | 布尔取反（用于 `when`） |
| `loop.done` / `loop.failed` | 循环终止信号 |

## 状态机执行模型

```
┌──────────┐  depth='deep'   ┌─────────────┐
│  triage  │────────────────▶│ full-review  │──┐ score>=80 ──▶ approve ──▶ __end__
│(decision)│                 └─────────────┘  └─ score<80  ──▶ request-changes ──▶ __end__
│          │────────────────▶│ quick-check │──────────────────▶ approve ──▶ __end__
└──────────┘ depth='shallow' └─────────────┘
```

- **Agent** 拥有执行权 — 读取步骤、执行动作、更新 context、评估 transitions
- **`writes`** 强制执行：步骤不能修改未声明的 context 键
- Transitions 顺序评估，第一个匹配的 `when` 获胜
- 到达 `__end__` 或 transitions 耗尽时，工作流终止

## 框架兼容性

| 框架 | 映射关系 |
|------|---------|
| LangGraph | Steps → nodes；transitions → 条件边；context → state |
| CrewAI Flows | Steps → `@listen`/`@router` 处理器；context → `self.state` |
| OpenAI Swarm | Steps → routines；`agent` steps → handoffs |
| Prefect | Steps → tasks；`parallel` → `asyncio.gather`；loop → retry |

## 发布

```bash
agentverse publish --file workflow.toml
# → 已发布 workflow myorg/pr-review-flow@0.1.0

# 以初始 context 运行
agentverse run --kind workflow --namespace myorg --name pr-review-flow \
  --context pr_url=https://github.com/org/repo/pull/42
```

## 相关文档

- [Manifest 格式总览](format_zh.md)
- [Soul Manifest](soul_zh.md)
- [Prompt Manifest](prompt_zh.md)
- [Agent Manifest](agent_zh.md)

