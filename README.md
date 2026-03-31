# AI Singularity

<div align="center">

**🤖 AI 账号与资源统一管理工具**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/Built%20with-Tauri%20v2-24C8D8?logo=tauri)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Backend-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/Frontend-React%20+%20TypeScript-61DAFB?logo=react)](https://react.dev)

*将分散的 AI 资源统一掌控，让每一次调用都物尽其用*

</div>

---

## 📖 项目简介

**AI Singularity** 是一款 **桌面 GUI + CLI 二合一** 的 AI 资源统一管理平台。  
目标是成为开发者桌面上的 **AI 控制中心**：管账号、管密钥、管模型、管工具、管配置，一站式搞定。

覆盖场景：
- **API Key / Session 账号管理**：OpenAI、Claude、Gemini、DeepSeek 等主流平台
- **AI 编码工具管理**：Claude Code、Codex、Gemini CLI、Aider、OpenCode 等 CLI 工具的配置统一切换
- **本地代理网关**：OpenAI 兼容代理，智能路由 + 自动降级 + 协议互转
- **MCP / Prompts / Skills 生态**：统一管理 MCP Server、系统提示词、AI 技能插件

### 解决的核心痛点

| 痛点 | 解决方式 |
|------|---------|
| API Key / Session 账号分散 | 统一加密存储，支持 Key 与 OAuth Session 双模式 |
| 各平台成本不透明 | 聚合看板，统一换算为 RMB/USD 横向对比 |
| AI 工具配置切换繁琐 | 50+ provider 预设，一键切换，系统托盘秒切 |
| 配额不透明，高级额度被后台任务浪费 | 配额感知路由 + 后台任务自动降级 |
| 没有告警 | 余额低、Key 过期、超预算主动通知 |
| MCP Server 各工具单独配置 | 统一 MCP 面板，跨工具双向同步 |
| 配置文件易损坏 | 原子写入（tmp + rename）+ 自动备份轮换 |

---

## ✨ 功能特性

### 🔑 1. 账号 & API Key 管理
- **双模式导入**：API Key 直录 + OAuth 2.0 浏览器授权（支持 Google/Anthropic Web Session）
- **本地加密存储**：Windows Credential Store / AES-256-GCM 降级方案
- **Key 状态检测**：有效性、过期时间、权限范围实时校验；403 封禁自动检测并跳过
- **批量导入**：.env 文件扫描、JSON 批量导入、旧版数据库热迁移
- **账号健康看板**：各平台账号配额剩余量、重置周期、最后同步时间一览

### 📊 2. 用量 & 账单聚合看板
- 实时同步各平台 Token 消耗、API 调用次数
- 统一换算成本（RMB / USD），支持自定义每模型单价
- 历史趋势图表（日 / 周 / 月），详细请求日志浏览
- 按项目 / 应用维度分组统计消费

### 🤖 3. 模型目录 & 能力矩阵
- 收录主流平台最新模型列表（持续更新）
- 模型能力标注：上下文长度、多模态支持、价格、速度
- 「最优模型推荐」：按任务类型（代码/写作/分析）自动推荐性价比最高的模型
- API 端点延迟测速：实测各 endpoint 响应速度，选择最快节点

### ⚡ 4. 智能路由 & 本地代理 (Proxy Gateway)
- 本地启动 **OpenAI 兼容** HTTP 代理端点，零侵入接入现有项目
- **全协议转换**：OpenAI ↔ Anthropic ↔ Gemini 三种格式互转
- **配额感知路由**：综合考虑账号类型(Ultra/Pro/Free)、当前配额剩余量、重置周期，自动选最优账号
- **后台任务自动降级**：识别低优先级请求（标题生成、摘要等），自动路由到 Flash/mini 模型，保护高级配额
- **失败自动重试 & 熔断降级**：429/401 毫秒级触发自动轮换，确保服务不中断
- **电路断路器 & 健康监控**：实时监控各 Provider 可用性
- 统一请求日志与 Token 实时统计

### 🛠️ 5. AI 编码工具管理 (CLI Tool Hub)
统一管理 Claude Code、Codex、Gemini CLI、Aider、OpenCode 等 AI 编码工具的配置：
- **50+ Provider 预设**：AWS Bedrock、NVIDIA NIM、各大社区中转，一键导入
- **一键切换**：主界面选择 provider → 启用；系统托盘直接点击，Claude Code 无需重启终端
- **拖拽排序**：自定义 provider 优先级顺序
- **跨工具同步**：Universal Provider 一次配置，同步到多个 CLI 工具
- **自定义共享配置片段**：提取并复用跨 provider 的公共配置（插件数据等）
- **Deep Link 协议** (`ais://`)：通过 URL 一键导入 Provider / MCP / Prompts 配置，方便社区分享

### 🔌 6. MCP Server 统一管理
- 统一面板管理各 AI 编码工具的 MCP Server，支持双向同步
- 模板库 + 自定义配置两种添加方式
- Deep Link 方式一键导入社区分享的 MCP 配置
- 按工具独立开关同步

### 📝 7. Prompts & Skills 管理
- **系统提示词管理**：Markdown 编辑器创建/编辑，一键同步到 CLAUDE.md / AGENTS.md / GEMINI.md
- **回填保护**：编辑激活中的 provider 时，从实际文件回填内容，防止覆盖丢失
- **Skills 管理**：从 GitHub Repo / ZIP 一键安装 AI Skills/Plugins，symlink 或文件复制两种模式

### 💬 8. 会话历史管理
- 跨工具浏览、搜索 Claude Code / Codex 等 AI 工具的对话历史
- 会话恢复与工作区文件预览
- 自动备份轮换（保留最近 10 份）

### 🔔 9. 告警 & 通知
- 余额低于自定义阈值告警
- API Key 即将过期提前提醒
- 月度用量超出预算通知
- 通知渠道：系统桌面通知、Webhook（企业微信 / 飞书 / 钉钉）、邮件

### ☁️ 10. 云端同步
- 自定义配置存储目录，支持同步到 Dropbox / OneDrive / iCloud
- WebDAV 服务器同步，适配 NAS 场景
- 多设备共享 provider / MCP / Prompts 配置

### 🖥️ 11. CLI 接口
```bash
# Key 管理
ais key list                          # 列出所有已录入的 Key
ais key add --platform openai         # 交互式添加新 Key
ais key check --platform anthropic    # 检测 Key 有效性

# 账号信息
ais balance                           # 查看所有账号余额汇总
ais balance --platform deepseek       # 指定平台余额

# 代理管理
ais proxy start --port 8080           # 启动本地 OpenAI 兼容代理
ais proxy stop                        # 停止代理
ais route list                        # 查看路由规则

# 模型信息
ais model list                        # 列出所有可用模型
ais model list --platform openai      # 按平台过滤
ais model compare gpt-4o claude-3-5   # 对比模型能力与价格
ais model speedtest                   # 测试各 endpoint 延迟

# 工具配置
ais provider switch <name>            # 切换 AI 编码工具的 provider
ais mcp list                          # 列出 MCP Server
ais mcp add <url>                     # 添加 MCP Server
```

---

## 🌐 支持平台

### AI API 平台

| 平台 | Key 管理 | OAuth Session | 余额查询 | 用量统计 | 模型列表 |
|------|:---:|:---:|:---:|:---:|:---:|
| OpenAI | ✅ | — | ✅ | ✅ | ✅ |
| Anthropic (Claude) | ✅ | ✅ | ✅ | ✅ | ✅ |
| Google Gemini | ✅ | ✅ | — | ✅ | ✅ |
| DeepSeek | ✅ | — | ✅ | ✅ | ✅ |
| 阿里云百炼 | ✅ | — | ✅ | ✅ | ✅ |
| 字节豆包 | ✅ | — | ✅ | ✅ | ✅ |
| Moonshot (Kimi) | ✅ | — | ✅ | ✅ | ✅ |
| 智谱 GLM | ✅ | — | ✅ | ✅ | ✅ |
| 自定义 OpenAI 兼容 | ✅ | — | — | — | ✅ |
| AWS Bedrock | ✅ | — | — | ✅ | ✅ |
| NVIDIA NIM | ✅ | — | — | ✅ | ✅ |

### AI 编码工具 (CLI Tool Hub)

| 工具 | Provider 管理 | MCP 同步 | Prompts 同步 | 会话历史 |
|------|:---:|:---:|:---:|:---:|
| Claude Code | ✅ | ✅ | ✅ | ✅ |
| OpenAI Codex | ✅ | ✅ | ✅ | ✅ |
| Gemini CLI | ✅ | ✅ | ✅ | — |
| Aider | ✅ | — | ✅ | — |
| OpenCode | ✅ | ✅ | ✅ | — |

---

## 🛠️ 技术栈

```
AI Singularity
├── 桌面应用 (Tauri v2)
│   ├── Frontend:   React 18 + TypeScript + Vite
│   ├── 样式:       Vanilla CSS / CSS Modules
│   ├── 状态管理:   Zustand
│   └── 数据同步:   TanStack Query v5
├── Core Engine (Rust — Tauri 后端)
│   ├── 加密存储:   keyring (系统 Keychain) / AES-256-GCM
│   ├── HTTP 代理:  hyper + axum（OpenAI 兼容 + 协议转换）
│   ├── 异步运行时: tokio
│   ├── 数据库:     SQLite (rusqlite) — 可同步数据
│   ├── 设备配置:   JSON — 设备级 UI 偏好（不参与同步）
│   └── 写入安全:   原子写入（tmp + rename）+ 自动备份轮换
└── CLI (独立二进制，共享 Core Engine)
    └── clap v4 (命令行解析 + Shell 补全生成)
```

### 存储分层设计

| 存储层 | 内容 | 同步策略 |
|-------|------|---------|
| SQLite (`~/.ai-singularity/data.db`) | Provider、MCP、Prompts、Skills、用量记录 | 支持云端同步 |
| JSON (`~/.ai-singularity/settings.json`) | 设备级 UI 偏好、窗口状态 | 仅本地 |
| 系统 Keychain | API Key、Session Token（敏感数据） | 不同步 |
| 备份 (`~/.ai-singularity/backups/`) | 自动轮换，保留最近 10 份 | — |

---

## 📁 项目结构

```
AI Singularity/
├── src-tauri/                      # Rust 后端
│   ├── src/
│   │   ├── main.rs                 # Tauri 入口
│   │   ├── cli.rs                  # CLI 入口 (clap)
│   │   ├── commands/               # Tauri IPC 命令层（按领域分组）
│   │   ├── services/               # 业务逻辑层
│   │   │   ├── provider.rs         # Provider CRUD + 切换 + 回填
│   │   │   ├── proxy.rs            # 本地代理 + 智能路由
│   │   │   ├── mcp.rs              # MCP Server 管理 + 同步
│   │   │   ├── session.rs          # 会话历史浏览
│   │   │   ├── notify.rs           # 告警通知
│   │   │   └── sync.rs             # 云端同步（WebDAV/本地目录）
│   │   ├── providers/              # 各平台 API 适配器
│   │   │   ├── openai.rs
│   │   │   ├── anthropic.rs
│   │   │   ├── gemini.rs
│   │   │   └── ...
│   │   ├── proxy/                  # 代理引擎
│   │   │   ├── server.rs           # axum HTTP 服务
│   │   │   ├── router.rs           # 配额感知路由引擎
│   │   │   ├── converter.rs        # OpenAI/Anthropic/Gemini 协议转换
│   │   │   └── circuit_breaker.rs  # 熔断器
│   │   ├── store/                  # 存储层
│   │   │   ├── keychain.rs         # 系统 Keychain
│   │   │   ├── database.rs         # SQLite DAO
│   │   │   └── atomic_write.rs     # 原子写入工具
│   │   └── deeplink/               # ais:// 协议处理
│   └── Cargo.toml
├── src/                            # React 前端
│   ├── components/
│   │   ├── Dashboard/              # 用量看板
│   │   ├── Keys/                   # Key + OAuth 账号管理
│   │   ├── Providers/              # AI 工具 Provider 管理
│   │   ├── Proxy/                  # 代理配置 + 路由规则
│   │   ├── Mcp/                    # MCP Server 管理
│   │   ├── Prompts/                # 系统提示词管理
│   │   ├── Skills/                 # Skills 管理
│   │   ├── Sessions/               # 会话历史浏览
│   │   ├── Models/                 # 模型目录 + 能力矩阵
│   │   └── Settings/               # 全局设置 + 云端同步
│   ├── stores/                     # Zustand 状态管理
│   ├── hooks/                      # 自定义 Hooks
│   ├── lib/
│   │   ├── api/                    # Tauri IPC 封装（类型安全）
│   │   └── query/                  # TanStack Query 配置
│   ├── locales/                    # 国际化（zh/en）
│   ├── config/                     # Provider 预设、MCP 模板
│   └── main.tsx
├── package.json
├── tauri.conf.json
└── README.md
```

---

## 🚀 开发路线图

### Phase 1 — MVP：Core 账号管理（2~3 周）
- [ ] 项目脚手架：Tauri v2 + React + Rust
- [ ] 加密 Key 存储（系统 Keychain + AES 降级）
- [ ] OAuth 2.0 浏览器授权流程（Google / Anthropic）
- [ ] OpenAI / Anthropic / DeepSeek 平台接入
- [ ] 账号健康看板（配额、余额、用量）
- [ ] CLI `key` / `balance` / `model` 基础命令
- [ ] 原子写入 + 自动备份轮换基础设施

### Phase 2 — 智能代理网关（2 周）
- [ ] 本地 axum 代理服务（OpenAI 兼容）
- [ ] OpenAI / Anthropic / Gemini 协议互转
- [ ] 配额感知路由引擎（配额剩余 + 账号等级 + 重置周期）
- [ ] 后台任务自动降级（识别低优先级请求）
- [ ] 熔断器 + 自动重试 + 健康监控
- [ ] 请求日志 + 实时 Token 统计

### Phase 3 — AI 工具生态（2 周）
- [ ] CLI Tool Hub（Claude Code / Codex / Gemini CLI / Aider 配置管理）
- [ ] 50+ Provider 预设库
- [ ] MCP Server 统一管理 + 跨工具双向同步
- [ ] Prompts 管理（CLAUDE.md / AGENTS.md 同步 + 回填保护）
- [ ] Skills 一键安装（GitHub / ZIP）
- [ ] 会话历史浏览器
- [ ] Deep Link 协议 `ais://`

### Phase 4 — 完整体验（2 周）
- [ ] 所有平台适配器（百炼、豆包、Kimi、GLM、Bedrock、NIM 等）
- [ ] API 端点延迟测速
- [ ] 告警系统（余额 / 用量 / Key 过期）
- [ ] 系统托盘常驻 + 桌面通知
- [ ] 云端同步（WebDAV / OneDrive / Dropbox）

### Phase 5 — 发布准备（1 周）
- [ ] 安装包制作（Windows MSI / macOS dmg / Linux AppImage + deb）
- [ ] Homebrew Cask 发布
- [ ] 一键安装脚本（curl/PowerShell）
- [ ] Docker 镜像（Headless 无头模式，支持 NAS 部署）
- [ ] 完整 CLI 文档 + Shell 自动补全（Bash/Zsh/PowerShell）
- [ ] 配置文件导入导出 + 版本迁移

---

## 🏗️ 核心架构

```
                    ┌──────────────────────────────────────┐
                    │      Frontend (React + TS)            │
                    │  Components → Hooks → TanStack Query  │
                    └──────────────┬───────────────────────┘
                                   │ Tauri IPC
                    ┌──────────────▼───────────────────────┐
                    │      Backend (Tauri + Rust)           │
                    │  Commands → Services → DAO → SQLite   │
                    └──────────────────────────────────────┘
                            │              │
               ┌────────────▼───┐    ┌────▼────────────────┐
               │ 系统 Keychain  │    │  axum Proxy Server   │
               │  API Key 加密  │    │  路由 / 协议转换       │
               └────────────────┘    └─────────────────────┘
```

**核心设计原则：**
- **SSOT（唯一真实来源）**：所有可同步数据存入 SQLite
- **原子写入**：tmp 文件 + rename，防止配置文件损坏
- **二层存储**：SQLite 存可同步数据，JSON 存设备级偏好
- **双向同步**：切换时写入实际文件，编辑激活 provider 时从实际文件回填
- **最小侵入**：卸载 App 后 CLI 工具仍正常运行

---

## 📄 开源协议

[MIT License](LICENSE) © 2026 AI Singularity Contributors
