# Agent Manifest（代理清单）

> [English](agent.md) · **中文**

**Agent（代理）** 制品是对自主 AI 代理的完整描述 ——
包括其人格（soul）、能力（可调用的技能）、通信协议、权限和记忆配置。
代理可以组合、共享，并通过 MCP、A2A 或 OpenAI Functions 直接部署。

## 最简示例

```toml
[package]
kind        = "agent"
namespace   = "myorg"
name        = "greeter"
description = "友好的问候代理"

[[agent.skills]]
namespace = "agentverse"
name      = "base-responder"
version   = ">=0.1.0"

[agent.protocols]
mcp = { enabled = true, version = "2024-11-05" }

[metadata]
tags    = ["agent", "greeting"]
license = "MIT"
```

## 完整示例

```toml
[package]
kind        = "agent"
namespace   = "myorg"
name        = "code-assistant"
description = "自主代码助手：代码审查、测试生成、重构建议"

# ── 人格（Soul）──────────────────────────────────────────────────────────────
[agent.soul]
namespace = "agentverse"
name      = "empathetic-counselor"
version   = ">=0.1.0"

# ── 技能（代理可调用的工具）──────────────────────────────────────────────────
[[agent.skills]]
namespace = "agentverse-ci"
name      = "code-reviewer"
version   = ">=0.1.0"
alias     = "review_code"           # 通过 MCP / OpenAI 工具调用暴露的名称

[[agent.skills]]
namespace = "agentverse-ci"
name      = "release-notes-writer"
version   = ">=0.1.0"
alias     = "write_release_notes"
optional  = true                    # 技能不可用时代理仍可继续工作

[[agent.skills]]
namespace = "agentverse-ci"
name      = "api-smoke-tester"
version   = ">=0.1.0"
alias     = "smoke_test"
optional  = true

# ── 提示词 ────────────────────────────────────────────────────────────────────
[[agent.prompts]]
namespace = "agentverse"
name      = "chain-of-thought"
version   = ">=0.1.0"
role      = "reasoning"             # 用于多步推理的默认提示词

# ── 协议 ─────────────────────────────────────────────────────────────────────
[agent.protocols]
mcp      = { enabled = true,  version = "2024-11-05" }
a2a      = { enabled = true,  version = "0.2.5" }     # Google Agent-to-Agent 标准
openai   = { enabled = true,  functions = true }
langchain = { enabled = false }

# ── 权限（最小权限原则）──────────────────────────────────────────────────────
[agent.permissions]
network = ["read"]           # network:read | network:write
fs      = ["read"]           # fs:read | fs:write
secrets = []                 # 代理可访问的密钥名称列表

# ── 记忆 ─────────────────────────────────────────────────────────────────────
[agent.memory]
context_window  = 128000
summarize_at    = 100000
long_term       = { enabled = true, backend = "pgvector" }
episodic        = { enabled = true, max_episodes = 100 }

# ── 模型偏好 ─────────────────────────────────────────────────────────────────
[agent.model]
preferred   = ["claude-3-5-sonnet", "gpt-4o", "gemini-1.5-pro"]
temperature = 0.2
max_tokens  = 4096

[metadata]
tags     = ["agent", "code-assistant", "mcp", "a2a", "developer"]
homepage = "https://github.com/myorg/agents"
license  = "MIT"

[metadata.openclaw]
emoji   = "🤖"
version = "0.1.0"
```

## 字段参考

### `[agent.soul]`

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `namespace` | string | — | Soul 制品的命名空间 |
| `name` | string | — | Soul 制品名称 |
| `version` | string | — | SemVer 约束 |

### `[[agent.skills]]`

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `namespace` | string | ✅ | 技能的命名空间 |
| `name` | string | ✅ | 技能名称 |
| `version` | string | ✅ | SemVer 版本约束 |
| `alias` | string | — | 通过 MCP / OpenAI Functions 暴露的工具名 |
| `optional` | boolean | — | 为 true 时，技能不可用不影响代理运行 |

### `[[agent.prompts]]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `namespace` | string | Prompt 制品的命名空间 |
| `name` | string | Prompt 制品名称 |
| `version` | string | SemVer 约束 |
| `role` | string | 代理使用此提示词的角色（`reasoning`、`system`、`tool-use`） |

### `[agent.protocols]`

| 协议 | 版本字段 | 说明 |
|------|---------|------|
| `mcp` | `2024-11-05` | Model Context Protocol（Anthropic 标准） |
| `a2a` | `0.2.5` | Agent-to-Agent（Google 标准） |
| `openai` | — | OpenAI 工具调用 / 函数调用格式 |
| `langchain` | — | LangChain 代理接口 |

### `[agent.permissions]`

| 字段 | 可选值 | 说明 |
|------|--------|------|
| `network` | `read`、`write` | 网络访问级别 |
| `fs` | `read`、`write` | 文件系统访问级别 |
| `secrets` | 密钥名称列表 | 代理可读取的密钥 |

### `[agent.memory]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `context_window` | integer | 最大 token 上下文长度 |
| `summarize_at` | integer | 触发自动摘要的 token 阈值 |
| `long_term.enabled` | boolean | 是否启用向量数据库长期记忆 |
| `long_term.backend` | string | `pgvector`、`pinecone`、`weaviate`、`qdrant` |
| `episodic.enabled` | boolean | 是否存储对话片段以供后续召回 |

## MCP 集成

`agent.skills` 中列出的技能会**自动注册为 MCP 工具**：

```json
{
  "mcpServers": {
    "code-assistant": {
      "command": "agentverse",
      "args": ["run", "--kind", "agent", "--namespace", "myorg", "--name", "code-assistant"],
      "env": { "AGENTVERSE_TOKEN": "${AGENTVERSE_TOKEN}" }
    }
  }
}
```

## A2A Agent Card

当 `a2a.enabled = true` 时，代理在 `/.well-known/agent.json` 发布 Agent Card：

```json
{
  "name": "code-assistant",
  "description": "自主代码助手代理",
  "version": "0.1.0",
  "url": "https://agentverse.example.com/agents/myorg/code-assistant",
  "capabilities": { "streaming": true, "pushNotifications": false },
  "skills": [
    { "id": "review_code",         "name": "代码审查",   "description": "分析 diff 或 PR" },
    { "id": "write_release_notes", "name": "发布日志撰写", "description": "从提交历史生成发布日志" }
  ]
}
```

## 标准兼容性

| 标准 | 支持 | 说明 |
|------|------|------|
| MCP 2024-11-05 | ✅ | 完整工具调用；技能自动注册 |
| Google A2A 0.2.5 | ✅ | Agent Card + 任务委托协议 |
| OpenAI Functions | ✅ | 从技能规格生成函数调用 JSON Schema |
| LangChain Agent | ✅ | 技能可包装为 `Tool` 对象 |
| CrewAI | ✅ | 可注册为 CrewAI 成员 |
| AutoGen | ✅ | 通过 OpenAI Functions 接口兼容 |

## 发布与运行

```bash
# 发布代理
agentverse publish --file agent.toml
# → 已发布 agent myorg/code-assistant@0.1.0

# 以 MCP 服务器模式运行
agentverse run --kind agent --namespace myorg --name code-assistant

# 获取代理详情（含 A2A Agent Card）
agentverse get --kind agent --namespace myorg --name code-assistant --format json
```

## 相关文档

- [Manifest 格式总览](format_zh.md)
- [Soul Manifest](soul_zh.md)
- [Prompt Manifest](prompt_zh.md)
- [Workflow Manifest](workflow_zh.md)

