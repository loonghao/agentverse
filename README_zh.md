<div align="center">

# 🌌 AgentVerse

**AI Agent 生态系统的通用超级市场**

*发布、发现、组合 AI 技能、Agent、工作流、人格和更多内容 — 一站满足。*

[![CI](https://github.com/loonghao/agentverse/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/agentverse/actions/workflows/ci.yml)
[![Release](https://github.com/loonghao/agentverse/actions/workflows/release-please.yml/badge.svg)](https://github.com/loonghao/agentverse/actions/workflows/release-please.yml)
[![Docker](https://ghcr-badge.egpl.dev/loonghao/agentverse/latest_tag?color=%2344cc11&ignore=latest&label=docker)](https://github.com/loonghao/agentverse/pkgs/container/agentverse)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

[English](README.md) · [部署指南](docs/deployment.md) · [使用指南](docs/usage.md)

</div>

---

## ✨ 什么是 AgentVerse？

AgentVerse 是一个开源、可自托管的 AI Agent 生态注册中心与市场。把它想象成 **AI 界的 npm** —— 但从底层设计就为了处理不仅仅是代码，而是 Agent 生态系统所需的全部产物：

| 类型 | 说明 | 示例 |
|------|------|------|
| 🔧 **Skill（技能）** | 可复用的能力和工具 | 网页爬虫工具、代码审查函数 |
| 🤖 **Agent（智能体）** | 具有定义能力的自主 AI | 客服 Agent、QA 工程师 Agent |
| 🔄 **Workflow（工作流）** | 多步骤编排流水线 | CI/CD 流水线、数据处理 DAG |
| 👤 **Soul（人格）** | 性格与人格配置 | 富有同理心的顾问人格 |
| 💬 **Prompt（提示词）** | 优化的提示模板 | 思维链提示、系统提示 |

**面向未来设计** —— 可扩展的产物模型意味着新类型可以注册，而不会破坏现有客户端。

## 🚀 快速开始

### 方式一：Docker Compose（推荐）

```bash
git clone https://github.com/loonghao/agentverse.git
cd agentverse
docker compose up -d
```

服务器现在可通过 `http://localhost:8080` 访问。

### 方式二：下载 CLI 二进制

```bash
# macOS / Linux
curl -fsSL https://github.com/loonghao/agentverse/releases/latest/download/agentverse-$(uname -m)-apple-darwin.tar.gz | tar -xz
./agentverse --help

# Windows (PowerShell)
irm https://github.com/loonghao/agentverse/releases/latest/download/agentverse-x86_64-pc-windows-msvc.zip -OutFile agentverse.zip
Expand-Archive agentverse.zip
```

### 方式三：Docker 镜像

```bash
docker pull ghcr.io/loonghao/agentverse:latest
docker run -d \
  -e DATABASE_URL=postgres://... \
  -e JWT_SECRET=your-secret \
  -p 8080:8080 \
  ghcr.io/loonghao/agentverse:latest
```

## 🎯 CLI 使用

```bash
# 搜索任何内容
agentverse search --query "代码审查" --kind skill

# 发布你的技能
agentverse publish --file skill.toml

# 获取特定产物
agentverse get --kind agent --namespace myorg --name code-reviewer

# 社交功能
agentverse like --kind skill --namespace python-tools --name linter
agentverse rate --kind workflow --namespace ops --name deploy --stars 5
```

## 🏗️ 架构

```
┌─────────────────────────────────────────────────────────────┐
│                   AgentVerse 平台                            │
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  REST API   │  │  GraphQL    │  │    MCP 协议         │ │
│  │  (OpenAPI)  │  │  接口       │  │  (AI Agent 原生)    │ │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
│         └────────────────┼──────────────────────┘           │
│                   ┌──────┴──────┐                           │
│                   │  核心逻辑   │                           │
│                   │  + Auth/JWT │                           │
│                   └──────┬──────┘                           │
│         ┌────────────────┼────────────────┐                 │
│  ┌──────┴──────┐  ┌──────┴──────┐  ┌──────┴──────┐         │
│  │ PostgreSQL  │  │    Redis    │  │    MinIO    │         │
│  │ + pgvector  │  │   缓存      │  │  产物存储   │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

**核心特性：**
- 🔒 **JWT 认证** 配合 Ed25519 签名产物
- 🔍 **全文 + 语义搜索**（pgvector 向量嵌入）
- 📦 **语义版本控制**，自动推断版本号升级
- 👥 **社交层**：评论、点赞、评分、Fork
- 🤖 **MCP 原生**：AI Agent 通过模型上下文协议交互
- 📊 **事件溯源**：完整的审计记录和分析

## 📖 文档

| 文档 | 说明 |
|------|------|
| [部署指南](docs/deployment.md) | Docker、Kubernetes、裸机部署 |
| [使用指南](docs/usage.md) | CLI 命令、API 示例、Manifest 格式 |
| [API 参考](http://localhost:8080/swagger-ui/) | 交互式 OpenAPI 文档（运行时） |

## 🛠️ 开发

```bash
git clone https://github.com/loonghao/agentverse.git
cd agentverse

# 启动开发依赖
docker compose up postgres redis minio -d

# 运行测试
just test

# 构建发布版本
just build-release

# 格式化和检查
just ci
```

## 📄 许可证

MIT 许可证 — 详见 [LICENSE](LICENSE) 文件。

