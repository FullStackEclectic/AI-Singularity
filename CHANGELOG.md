# Changelog

All notable changes to AI Singularity will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Unified account management with support for 15+ IDE platforms (Claude Code, Codex, Gemini CLI, Cursor, Windsurf, Kiro, Trae, Zed, Qoder, CodeBuddy, WorkBuddy, VSCode, GitHub Copilot, Aider, OpenCode)
- Local OpenAI-compatible proxy gateway with intelligent routing, circuit breaker, and protocol conversion (OpenAI ↔ Anthropic ↔ Gemini)
- Gemini native SSE streaming support
- Anthropic ↔ Gemini bidirectional protocol conversion
- Provider preset library with official and relay options for all supported tools
- MCP Server unified management with cross-tool sync
- System Prompts management with auto-sync to CLAUDE.md / AGENTS.md / GEMINI.md / .aider.conf.yml
- Skills management (install from GitHub, update, uninstall)
- Session history browser for Claude Code, Codex, Gemini CLI, Aider, OpenCode
- Balance dashboard with multi-platform aggregation (OpenAI, DeepSeek, Moonshot, Zhipu, Aliyun, MiniMax, Bytedance, SiliconFlow, OpenRouter)
- Alert notification channels: OS native, Feishu webhook, DingTalk webhook (with HMAC-SHA256 signing), WeCom webhook, SMTP email
- WebDAV cloud sync for configuration backup
- Auto account switching with quota-aware routing
- Device fingerprint management
- Wakeup scheduler for automated task execution
- Deep link support (`ais://`) for importing Provider, MCP, and Prompts configurations
- CLI tool (`ais`) with key management, balance query, proxy control, MCP management, model directory, and speedtest
- GitHub Actions CI/CD: cross-platform tests, Tauri release builds, weekly security audit
- IP access control with blacklist/whitelist and rate limiting
- User token management for proxy gateway access control
- Model directory with 200+ models and pricing data
- Token calculator with multi-platform cost comparison
- MFA vault (TOTP) with Base32 support
- Announcement center
- Floating account cards
- Web report page

### Fixed
- Deep link scheme mismatch (`aisingularity://` → `ais://`)
- Router test assertion corrected to match actual priority-mode scheduling behavior

### Security
- API keys stored in system Keychain (Windows Credential Store / macOS Keychain / libsecret)
- AES-256-GCM fallback encryption for platforms without system Keychain
- Proxy gateway IP access control with CIDR support
- Rate limiting per IP/token (120 RPM default)

## [0.1.0] - 2026-05-14

### Added
- Initial release
