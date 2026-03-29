# Manifest 格式说明

> [English](format.md) · **中文**

`agentverse.toml` 清单文件用于向注册中心描述您的制品（artifact）。`agentverse publish` 命令读取此文件来创建或更新制品。

## 完整示例

```toml
# agentverse.toml

# ── 包身份标识 ────────────────────────────────────────────────────────────────
[package]
kind        = "skill"           # skill | agent | workflow | soul | prompt
namespace   = "myorg"           # 您的用户名或组织名
name        = "code-linter"     # 在 kind+namespace 内唯一
description = "基于 AST 分析的 Python 代码检查工具"

# ── 能力声明 ──────────────────────────────────────────────────────────────────
[capabilities]
input_modalities  = ["text", "json"]
output_modalities = ["text", "json"]
protocols         = ["mcp", "openai-function"]
permissions       = ["network:read", "fs:read"]
max_tokens        = 4096

# ── 依赖声明 ──────────────────────────────────────────────────────────────────
[dependencies]
"python-tools/ast-parser" = ">=1.0.0"
"myorg/base-linter"       = "^2.1.0"

# ── 元数据 ────────────────────────────────────────────────────────────────────
[metadata]
tags     = ["python", "linting", "ast", "automation"]
homepage = "https://github.com/myorg/code-linter"
license  = "MIT"
```

## 字段参考

### `[package]`

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `kind` | string | ✅ | 制品类型：`skill`、`agent`、`workflow`、`soul` 或 `prompt` |
| `namespace` | string | ✅ | 所有者命名空间（您的用户名或组织名） |
| `name` | string | ✅ | 制品名称（小写字母、连字符、数字） |
| `description` | string | — | 简短描述（显示在搜索结果中） |

### `[capabilities]`

存储在清单中，用于基于能力的检索发现。

| 字段 | 类型 | 说明 |
|------|------|------|
| `input_modalities` | string[] | 接受的输入类型：`text`、`json`、`image`、`audio`、`file` |
| `output_modalities` | string[] | 产生的输出类型 |
| `protocols` | string[] | 支持的协议：`mcp`、`openai-function`、`a2a` 等 |
| `permissions` | string[] | 所需权限：`network:read`、`fs:read`、`fs:write` 等 |
| `max_tokens` | integer | 最大 token 上下文长度 |

### `[dependencies]`

声明本制品依赖的其他 AgentVerse 制品。

```toml
[dependencies]
"python-tools/ast-parser" = ">=1.0.0, <2.0.0"
"myorg/base-skill"        = "^1.2.0"
```

版本约束语法遵循 SemVer 区间规范。

### `[metadata]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `tags` | string[] | 可搜索的标签 |
| `homepage` | string | 项目主页或 GitHub 仓库 URL |
| `license` | string | SPDX 许可证标识符（如 `MIT`、`Apache-2.0`） |

## 制品种类

| Kind | 用途 |
|------|------|
| `skill` | 可复用的工具或能力（代码审查、网页抓取等） |
| `agent` | 具有人格定义的自主 AI 代理 |
| `workflow` | 多步骤编排流水线或 DAG |
| `soul` | 代理的个性 / 人格配置 |
| `prompt` | 优化的提示词模板或思维链 |

---

## 各种类专属字段

每种制品类型支持一个可选的同名 TOML 节，用于描述更丰富的元数据。

### `[soul]` — 人格与个性

当 `kind = "soul"` 时添加 `[soul]` 节：

```toml
[soul]
tone           = "empathetic"       # empathetic | formal | casual | direct | playful
language_style = "conversational"   # conversational | technical | academic | simple

[soul.persona]
name       = "Alex"
background = "拥有 10 年正念与 CBT 经验的资深人生教练"
greeting   = "你好，我在这里聆听。今天有什么想聊的？"

[[soul.values]]
name        = "empathy"
description = "在提供解决方案之前，始终先认可对方的感受"

[[soul.constraints]]
rule    = "no_professional_advice"
message = "我不是持证专业人士，严重问题请咨询专业人士。"
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `tone` | string | 沟通风格（`empathetic`、`formal`、`casual`、`direct`、`playful`） |
| `language_style` | string | 词汇风格（`conversational`、`technical`、`academic`、`simple`） |
| `persona.name` | string | 向用户展示的角色名 |
| `persona.greeting` | string | 开场白 |
| `values[].name` | string | 核心价值观标识符 |
| `constraints[].rule` | string | 机器可读的约束 ID |
| `constraints[].message` | string | 注入 system prompt 的人类可读说明 |

> **OpenClaw Soul Agents**：带有 `[metadata.openclaw]` 的 Soul 制品会自动兼容
> OpenClaw Soul Agent 运行时，运行时读取 `soul.tone`、`soul.values`、`soul.constraints`
> 来构造 LLM system prompt。

详细字段参考：[Soul Manifest 文档](soul_zh.md)

---

### `[prompt]` — 提示词模板与推理链

当 `kind = "prompt"` 时添加 `[prompt]` 节：

```toml
[prompt]
template_engine = "jinja2"                      # jinja2 | handlebars | mustache | plain
input_variables = ["problem", "domain", "style"]

[prompt.system]
text = "你是一位专业的问题解决专家，请逐步分析问题。"

[prompt.user]
text = "问题：{{problem}}\n领域：{{domain}}\n\n让我们一步一步来思考："

[[prompt.examples]]
input  = { problem = "37 是质数吗？", domain = "数学" }
output = "第 1 步：37 是奇数，不被 2 整除…\n答案：37 是质数。✓"

[prompt.output_format]
type   = "markdown"
schema = "编号步骤 + 最终答案"

[prompt.model_hints]
preferred   = ["gpt-4o", "claude-3-5-sonnet"]
temperature = 0.2
max_tokens  = 2048
```

**标准兼容性：** `system`/`user` 直接映射到 OpenAI Chat 和 Anthropic Messages API 角色。
`input_variables` 与 LangChain `PromptTemplate` 完全对齐。

详细字段参考：[Prompt Manifest 文档](prompt_zh.md)

---

### `[workflow]` — 多步骤 DAG 流水线

当 `kind = "workflow"` 时添加 `[workflow]` 节：

```toml
[workflow]
trigger = "github_pr"    # github_pr | schedule | webhook | manual

[[workflow.steps]]
id         = "code-review"
name       = "AI 代码审查"
kind       = "skill"
namespace  = "agentverse-ci"
artifact   = "code-reviewer"
version    = ">=0.1.0"
inputs     = { diff = "{{trigger.pr_url}}", rules = ["security", "correctness"] }
on_error   = "fail"       # fail | warn | continue | retry

[[workflow.steps]]
id         = "release-notes"
name       = "起草发布日志"
kind       = "skill"
namespace  = "agentverse-ci"
artifact   = "release-notes-writer"
version    = ">=0.1.0"
depends_on = ["code-review"]
inputs     = { repo = "{{trigger.repo}}", from_ref = "{{trigger.base_sha}}" }
on_error   = "continue"

[workflow.outputs]
review_summary = "{{steps.code-review.outputs.summary}}"
release_draft  = "{{steps.release-notes.outputs.notes}}"
```

**DAG 执行：** 没有 `depends_on` 的步骤默认**并行**运行。

**标准兼容性：** 可导出为 GitHub Actions（`--format github-actions`）、
Argo Workflows（`--format argo-workflow`）或 Prefect（`--format prefect`）。

详细字段参考：[Workflow Manifest 文档](workflow_zh.md)

---

### `[agent]` — 自主 AI 代理

当 `kind = "agent"` 时添加 `[agent]` 节：

```toml
[agent.soul]
namespace = "agentverse"
name      = "empathetic-counselor"
version   = ">=0.1.0"

[[agent.skills]]
namespace = "agentverse-ci"
name      = "code-reviewer"
version   = ">=0.1.0"
alias     = "review_code"    # 通过 MCP 暴露的工具名

[agent.protocols]
mcp    = { enabled = true, version = "2024-11-05" }
a2a    = { enabled = true, version = "0.2.5" }
openai = { enabled = true, functions = true }

[agent.permissions]
network = ["read"]
fs      = ["read"]
secrets = []

[agent.memory]
context_window = 128000
long_term      = { enabled = true, backend = "pgvector" }

[agent.model]
preferred   = ["claude-3-5-sonnet", "gpt-4o"]
temperature = 0.2
max_tokens  = 4096
```

**MCP：** `agent.skills` 中列出的技能会自动注册为 MCP 工具定义。

**A2A：** 当 `a2a.enabled = true` 时，代理在 `/.well-known/agent.json` 发布 Agent Card。

详细字段参考：[Agent Manifest 文档](agent_zh.md)

---

## 内容文件（content.json）

在 `agentverse.toml` 同目录下提供 `content.json`，包含制品的实际内容：

```json
{
  "schema_version": "1.0",
  "kind": "skill",
  "system_prompt": "你是一个 Python 代码检查工具...",
  "config": {
    "rules": ["E501", "F401"],
    "max_line_length": 88
  }
}
```

`content.json` 中的 `kind` 字段**必须与**清单中的 `[package].kind` 一致。
CLI 会自动读取与清单同目录下的 `content.json`。

## OpenClaw 扩展

AgentVerse 支持 **OpenClaw** 元数据标准，在 `[metadata.openclaw]` 下声明：

```toml
[metadata]
tags     = ["python", "linting"]
homepage = "https://github.com/myorg/code-linter"
license  = "MIT"

[metadata.openclaw]
name        = "Python 代码检查器"
description = "基于 AST 分析的 Python 代码检查工具"
version     = "1.0.0"
author      = "myorg"

  [[metadata.openclaw.commands]]
  name        = "lint"
  description = "检查 Python 源文件"

    [[metadata.openclaw.commands.arguments]]
    name        = "files"
    description = "待检查的 Python 文件列表"
    required    = true
```

带有 OpenClaw 元数据发布的技能自动兼容 [ClawHub](https://clawhub.dev) 技能注册中心。

## 版本递增

`--bump` 参数或服务端 `default_bump` 配置控制版本策略：

| 模式 | 之前 | 之后 |
|------|------|------|
| `patch` | `1.2.0` | `1.2.1` |
| `minor` | `1.2.0` | `1.3.0` |
| `major` | `1.2.0` | `2.0.0` |

首次发布总是从 `0.1.0` 开始。

## 校验规则

| 字段 | 约束 |
|------|------|
| `kind` | 必须是：`skill`、`agent`、`workflow`、`soul`、`prompt` 之一 |
| `namespace` | 小写字母数字 + 连字符；必须与您的用户名/组织名匹配 |
| `name` | 小写字母数字 + 连字符；在 kind + namespace 内唯一 |
| `protocols` | 支持：`mcp`、`openai-function`、`a2a`、`langchain` |
| `permissions` | 支持：`network:read`、`network:write`、`fs:read`、`fs:write` |

