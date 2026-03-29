# Soul Manifest（人格清单）

> [English](soul.md) · **中文**

**Soul（人格）** 定义了 AI 代理的个性、语调、核心价值观和行为约束。
Soul 可以组合、版本化，并在 AgentVerse 注册中心中共享。

## 最简示例

```toml
[package]
kind        = "soul"
namespace   = "myorg"
name        = "developer-buddy"
description = "友好、精确的开发者伙伴人格"

[soul]
tone           = "casual"
language_style = "technical"

[metadata]
tags    = ["soul", "developer", "friendly"]
license = "MIT"
```

## 完整示例

```toml
[package]
kind        = "soul"
namespace   = "myorg"
name        = "empathetic-counselor"
description = "面向支持型代理的温暖共情辅导员人格"

# ── 个性设置 ──────────────────────────────────────────────────────────────────
[soul]
tone           = "empathetic"
language_style = "conversational"

[soul.persona]
name       = "Alex"
background = "拥有 10 年正念与认知行为疗法（CBT）经验的资深人生教练"
greeting   = "你好，我在这里聆听。今天有什么想聊的？"
avatar_url = "https://example.com/avatars/alex.png"

# ── 核心价值观 ────────────────────────────────────────────────────────────────
[[soul.values]]
name        = "empathy"
description = "在提供解决方案之前，始终先认可对方的感受"
priority    = 1

[[soul.values]]
name        = "non-judgment"
description = "避免评价性语言；接受用户的观点为有效"
priority    = 2

[[soul.values]]
name        = "active-listening"
description = "在回应之前，先复述用户所说的内容"
priority    = 3

# ── 约束（硬性规则）──────────────────────────────────────────────────────────
[[soul.constraints]]
rule    = "no_professional_advice"
message = "我不是持证专业人士。对于严重问题，请咨询专业人士。"

[[soul.constraints]]
rule    = "no_absolute_promises"
message = "我会尽力而为，但无法做出绝对保证。"

# ── 组合（继承其他 Soul）─────────────────────────────────────────────────────
[[soul.extends]]
namespace = "agentverse"
name      = "base-professional"
version   = ">=0.1.0"
priority  = 0             # 优先级越低，越容易被当前 Soul 覆盖

[metadata]
tags     = ["soul", "empathy", "counseling", "support", "openclaw"]
homepage = "https://github.com/myorg/souls"
license  = "MIT"

[metadata.openclaw]
emoji   = "🌿"
version = "0.1.0"
```

## 字段参考

### `[soul]`

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `tone` | string | ✅ | `empathetic`、`formal`、`casual`、`direct`、`playful` |
| `language_style` | string | — | `conversational`、`technical`、`academic`、`simple` |

### `[soul.persona]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | string | 展示给用户的角色名 |
| `background` | string | 注入 system prompt 的背景故事 |
| `greeting` | string | 开场白 |
| `avatar_url` | string | 头像图片 URL |

### `[[soul.values]]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | string | 价值观标识符（用于 system prompt 注入） |
| `description` | string | 该价值观在行为中的体现方式 |
| `priority` | integer | 数字越小，优先级越高 |

### `[[soul.constraints]]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `rule` | string | 机器可读的约束 ID |
| `message` | string | 注入 system prompt 的人类可读说明 |

### `[[soul.extends]]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `namespace` | string | 父 Soul 的命名空间 |
| `name` | string | 要继承的父 Soul 名称 |
| `version` | string | 父 Soul 的 SemVer 约束 |
| `priority` | integer | 合并优先级（0 = 最低，会被当前 Soul 字段覆盖） |

## 内容文件（content.json）

```json
{
  "schema_version": "1.0",
  "kind": "soul",
  "system_prompt": "你是 Alex，一位温暖共情的人生教练。积极倾听，反映用户的情绪，轻柔地引导他们走向清晰。不评判，不催促。",
  "soul": {
    "tone": "empathetic",
    "language_style": "conversational",
    "persona": {
      "name": "Alex",
      "greeting": "你好，我在这里聆听。今天有什么想聊的？"
    },
    "values": ["empathy", "non-judgment", "active-listening"],
    "constraints": [
      "永远不提供医疗或法律建议",
      "对于严重问题，始终建议寻求专业帮助"
    ]
  }
}
```

## OpenClaw Soul Agent 集成

带有 `[metadata.openclaw]` 的 Soul 可以被 OpenClaw Soul Agent 运行时直接消费：

```yaml
# openclaw-config.yaml
soul:
  source: agentverse
  namespace: myorg
  name: empathetic-counselor
  version: "0.1.0"
agent:
  system_prompt: "{{soul.system_prompt}}"
  tone: "{{soul.tone}}"
  constraints: "{{soul.constraints}}"
```

## 发布

```bash
agentverse publish --file soul.toml
# → 已发布 soul myorg/empathetic-counselor@0.1.0
```

## Soul 组合

多个 Soul 可以合并。后面的条目在字段级别覆盖前面的：

```bash
# 合并两个 Soul — empathetic-counselor 具有更高优先级
agentverse agent compose \
  --soul agentverse/base-professional \
  --soul myorg/empathetic-counselor
```

## 相关文档

- [Manifest 格式总览](format_zh.md)
- [Prompt Manifest](prompt_zh.md)
- [Workflow Manifest](workflow_zh.md)
- [Agent Manifest](agent_zh.md)

