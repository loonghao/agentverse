# Prompt Manifest（提示词清单）

> [English](prompt.md) · **中文**

**Prompt（提示词）** 制品存储版本化的、可复用的提示词模板 ——
它是基于 LLM 的推理、指令遵循和结构化输出生成的基础构建块。

## 最简示例

```toml
[package]
kind        = "prompt"
namespace   = "myorg"
name        = "summariser"
description = "将文档总结为要点列表"

[prompt]
input_variables = ["document"]

[prompt.user]
text = "请将以下内容总结为 5 条要点：\n\n{{document}}"

[metadata]
tags    = ["prompt", "summarisation"]
license = "MIT"
```

## 完整示例

```toml
[package]
kind        = "prompt"
namespace   = "myorg"
name        = "chain-of-thought"
description = "多步推理模板——支持零样本 CoT、少样本 CoT、思维树"

[prompt]
template_engine = "jinja2"
input_variables = ["problem", "domain", "style"]

[prompt.system]
text = """
你是一位专业的问题解决专家。收到问题后，在得出最终答案之前，
请将其分解为清晰的、编号的推理步骤。
逐步思考，展示你的过程。
"""

[prompt.user]
text = """
问题：{{problem}}
领域：{{domain}}
推理风格：{{style | default("zero-shot-cot")}}

让我们一步一步来分析：
"""

[[prompt.examples]]
input  = { problem = "37 是质数吗？", domain = "数学" }
output = """
第 1 步：37 是奇数，不被 2 整除。
第 2 步：3+7=10，不被 3 整除。
第 3 步：不以 0 或 5 结尾，不被 5 整除。
第 4 步：√37 ≈ 6.1，已检查所有 ≤6 的质数。
答案：37 是质数。✓
"""

[[prompt.examples]]
input  = { problem = "解释天空为什么是蓝色的", domain = "物理" }
output = """
第 1 步：阳光包含所有可见波长。
第 2 步：瑞利散射使短波长（蓝光）散射更多。
第 3 步：我们的眼睛从所有天空方向看到散射的蓝光。
答案：天空呈蓝色是因为瑞利散射效应。✓
"""

[prompt.output_format]
type   = "markdown"
schema = "编号步骤 + 最终答案"
strict = false

[prompt.model_hints]
preferred   = ["gpt-4o", "claude-3-5-sonnet", "gemini-1.5-pro"]
temperature = 0.2
max_tokens  = 2048

[metadata]
tags     = ["prompt", "chain-of-thought", "reasoning", "few-shot", "cot"]
homepage = "https://github.com/myorg/prompts"
license  = "MIT"
```

## 字段参考

### `[prompt]`

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `template_engine` | string | — | `jinja2`（默认）、`handlebars`、`mustache`、`plain` |
| `input_variables` | string[] | — | 调用方必须提供的变量名列表 |

### `[prompt.system]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `text` | string | 在对话前注入的系统角色内容（LLM system prompt） |

### `[prompt.user]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `text` | string | 用户轮次模板；使用 `{{变量名}}` 占位符语法 |

### `[[prompt.examples]]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `input` | object | 变量名到值的键值映射 |
| `output` | string | 期望的模型输出 |

### `[prompt.output_format]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `type` | string | `markdown`、`json`、`plain`、`xml` |
| `schema` | string | 输出结构的自由文本描述 |
| `strict` | boolean | 为 true 时，不符合格式的输出将被重试 |

### `[prompt.model_hints]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `preferred` | string[] | 偏好模型 ID 的有序列表 |
| `temperature` | float | 采样温度（0.0–2.0） |
| `max_tokens` | integer | 输出 token 上限 |

## 内容文件（content.json）

```json
{
  "schema_version": "1.0",
  "kind": "prompt",
  "template_engine": "jinja2",
  "input_variables": ["problem", "domain"],
  "system": "你是一位专业的问题解决专家，请逐步分析问题。",
  "user": "问题：{{problem}}\n领域：{{domain}}\n\n让我们一步一步来思考：",
  "examples": [
    {
      "input": { "problem": "37 是质数吗？", "domain": "数学" },
      "output": "第 1 步：37 是奇数…\n答案：37 是质数。✓"
    }
  ],
  "output_format": { "type": "markdown" },
  "model_hints": { "preferred": ["gpt-4o", "claude-3-5-sonnet"], "temperature": 0.2 }
}
```

## 标准兼容性

| 标准 | 使用方式 |
|------|---------|
| OpenAI Chat 格式 | `system.text` → system 角色；`user.text` → user 角色 |
| Anthropic Messages API | 与 OpenAI 相同的角色映射 |
| LangChain PromptTemplate | `input_variables` 完全对齐 |
| LlamaIndex | `PromptTemplate(template=prompt.user.text)` 直接加载 |
| DSPy | 作为 `dspy.ChainOfThought` 签名使用 |

## 提示词变体

| 风格 | 说明 |
|------|------|
| `zero-shot-cot` | 仅在用户消息末尾追加"让我们一步一步来思考" |
| `few-shot-cot` | 在问题前包含有解题过程的 `examples` |
| `tot` | 思维树：探索多条推理路径 |
| `self-ask` | 代理自行生成并回答子问题 |
| `react` | ReAct 模式：推理（Reason）与行动（Act）交替进行 |

## 发布与使用

```bash
# 发布
agentverse publish --file prompt.toml
# → 已发布 prompt myorg/chain-of-thought@0.1.0

# 以 JSON 格式获取（程序化使用）
agentverse get --kind prompt --namespace myorg --name chain-of-thought --format json
```

## 相关文档

- [Manifest 格式总览](format_zh.md)
- [Soul Manifest](soul_zh.md)
- [Workflow Manifest](workflow_zh.md)
- [Agent Manifest](agent_zh.md)

