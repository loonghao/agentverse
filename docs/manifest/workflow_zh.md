# Workflow Manifest（工作流清单）

> [English](workflow.md) · **中文**

**Workflow（工作流）** 制品定义了一个多步骤的有向无环图（DAG）流水线，
将 AgentVerse 技能、代理和外部命令组合成一个可复现的执行序列。
工作流是声明式的、版本化的，并可导出为主流编排工具的格式。

## 最简示例

```toml
[package]
kind        = "workflow"
namespace   = "myorg"
name        = "quick-review"
description = "对 PR 运行 AI 代码审查"

[workflow]
trigger = "manual"

[[workflow.steps]]
id        = "review"
kind      = "skill"
namespace = "agentverse-ci"
artifact  = "code-reviewer"
version   = ">=0.1.0"
inputs    = { diff = "{{trigger.diff_url}}" }

[metadata]
tags    = ["workflow", "code-review"]
license = "MIT"
```

## 完整示例

```toml
[package]
kind        = "workflow"
namespace   = "myorg"
name        = "ci-review-pipeline"
description = "完整 CI：代码分析 → 安全扫描 → 发布日志 → 通知"

[workflow]
trigger     = "github_pr"
timeout     = "30m"
concurrency = { max = 3, cancel_in_progress = false }

# ── 步骤 1：并行运行代码审查 + 安全扫描 ──────────────────────────────────────
[[workflow.steps]]
id        = "code-review"
name      = "AI 代码审查"
kind      = "skill"
namespace = "agentverse-ci"
artifact  = "code-reviewer"
version   = ">=0.1.0"
inputs    = { diff = "{{trigger.pr_url}}", rules = ["correctness", "performance"] }
on_error  = "fail"
retry     = { max_attempts = 2, delay = "30s" }
timeout   = "5m"

[[workflow.steps]]
id         = "security-scan"
name       = "安全扫描"
kind       = "skill"
namespace  = "agentverse-ci"
artifact   = "code-reviewer"
version    = ">=0.1.0"
depends_on = []                        # 与 code-review 并行执行
inputs     = { diff = "{{trigger.pr_url}}", rules = ["security"] }
on_error   = "warn"
timeout    = "5m"

# ── 步骤 2：发布日志（依赖 code-review）─────────────────────────────────────
[[workflow.steps]]
id         = "release-notes"
name       = "起草发布日志"
kind       = "skill"
namespace  = "agentverse-ci"
artifact   = "release-notes-writer"
version    = ">=0.1.0"
depends_on = ["code-review"]
inputs     = { repo = "{{trigger.repo}}", from_ref = "{{trigger.base_sha}}", to_ref = "{{trigger.head_sha}}" }
on_error   = "continue"

# ── 步骤 3：通知（依赖所有步骤）─────────────────────────────────────────────
[[workflow.steps]]
id         = "notify"
name       = "发布 PR 评论"
kind       = "http"
depends_on = ["code-review", "security-scan", "release-notes"]
url        = "{{trigger.comments_url}}"
method     = "POST"
headers    = { Authorization = "Bearer {{env.GITHUB_TOKEN}}" }
body       = { body = "**审查完成**\n\n{{steps.code-review.outputs.summary}}" }
on_error   = "warn"

[workflow.outputs]
review_summary = "{{steps.code-review.outputs.summary}}"
security_score = "{{steps.security-scan.outputs.score}}"
release_draft  = "{{steps.release-notes.outputs.notes}}"

[metadata]
tags     = ["workflow", "ci", "pipeline", "dag", "code-review", "security"]
homepage = "https://github.com/myorg/workflows"
license  = "MIT"
```

## 字段参考

### `[workflow]`

| 字段 | 类型 | 说明 |
|------|------|------|
| `trigger` | string | `github_pr`、`schedule`、`webhook`、`manual` |
| `timeout` | string | 全局工作流超时时间（如 `30m`、`2h`） |
| `concurrency.max` | integer | 最大并行工作流实例数 |
| `concurrency.cancel_in_progress` | bool | 新运行启动时是否取消旧运行 |

### `[[workflow.steps]]`

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | ✅ | 唯一步骤标识符；用于 `depends_on` |
| `name` | string | — | 人类可读的步骤名称 |
| `kind` | string | ✅ | `skill`、`agent`、`shell`、`http` |
| `namespace` | string | 条件 | `skill`/`agent` 类型必填 |
| `artifact` | string | 条件 | 命名空间内的制品名称 |
| `version` | string | 条件 | SemVer 约束（如 `>=0.1.0`） |
| `depends_on` | string[] | — | 必须先完成的步骤 ID；空则并行 |
| `inputs` | object | — | 输入绑定；支持 `{{trigger.*}}` / `{{steps.*}}` |
| `on_error` | string | — | `fail`（默认）、`warn`、`continue`、`retry` |
| `retry` | object | — | 自动重试的 `max_attempts` 和 `delay` |
| `timeout` | string | — | 单步超时时间（覆盖工作流级别） |

### `[workflow.outputs]`

使用 `{{steps.<id>.outputs.<key>}}` 将步骤输出绑定为工作流级别的命名结果。

## 步骤类型

| 类型 | 说明 |
|------|------|
| `skill` | 调用 AgentVerse skill 制品 |
| `agent` | 委托给 AgentVerse agent 制品 |
| `shell` | 运行 shell 命令（需要 `command` 字段） |
| `http` | 发起 HTTP 请求（需要 `url` 字段） |

## 模板变量

| 变量 | 说明 |
|------|------|
| `{{trigger.pr_url}}` | PR URL（github_pr 触发器） |
| `{{trigger.repo}}` | `owner/repo` 格式的仓库名 |
| `{{trigger.base_sha}}` | 基准提交 SHA |
| `{{trigger.head_sha}}` | 头部提交 SHA |
| `{{steps.<id>.outputs.<key>}}` | 上一步骤的输出值 |
| `{{env.VARIABLE_NAME}}` | 环境变量 |

## DAG 执行规则

```
步骤 A ──┐
         ├──→ 步骤 C（依赖 A 和 B）──→ 步骤 D
步骤 B ──┘
```

- **无 `depends_on`** → 与其他无依赖步骤**并行**执行
- **有 `depends_on`** → 等待所有依赖步骤成功后再执行
- **`on_error: fail`** → 步骤失败时终止整个工作流
- **`on_error: continue`** → 步骤失败时跳过并继续后续步骤

## 导出格式

```bash
# 导出为 GitHub Actions 工作流
agentverse get --kind workflow --namespace myorg --name ci-review-pipeline \
  --format github-actions > .github/workflows/ci-review.yml

# 导出为 Argo WorkflowTemplate
agentverse get --kind workflow --namespace myorg --name ci-review-pipeline \
  --format argo-workflow > ci-review.argo.yaml

# 导出为 Prefect 流
agentverse get --kind workflow --namespace myorg --name ci-review-pipeline \
  --format prefect > ci_review_flow.py
```

## 标准兼容性

| 标准 | 导出参数 | 说明 |
|------|---------|------|
| GitHub Actions | `github-actions` | 步骤映射到 `jobs.<id>.steps` |
| Argo Workflows | `argo-workflow` | 映射到 `WorkflowTemplate` CRD |
| Prefect | `prefect` | 生成 `@flow` Python 函数 |
| Apache Airflow | `airflow` | 生成 `DAG` Python 模块 |
| LangGraph | `langgraph` | 生成 `StateGraph` |

## 发布

```bash
agentverse publish --file workflow.toml
# → 已发布 workflow myorg/ci-review-pipeline@0.1.0
```

## 相关文档

- [Manifest 格式总览](format_zh.md)
- [Soul Manifest](soul_zh.md)
- [Prompt Manifest](prompt_zh.md)
- [Agent Manifest](agent_zh.md)

