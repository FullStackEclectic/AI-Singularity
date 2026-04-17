# Wakeup 官方客户端版本模式矩阵

## 1. 平台差异盘点

| 平台族 | `official_stable` | `official_preview` | `official_legacy` | 说明 |
| --- | --- | --- | --- | --- |
| `gemini` | 支持 | 支持 | 支持 | 有稳定/预览/旧版链路 |
| `codex` | 支持 | 支持 | 支持 | 有稳定/预览/旧版链路 |
| 其他平台 | 不支持 | 不支持 | 支持 | 统一走兼容链路 |

## 2. 客户端版本 -> 运行参数映射

### Gemini

- `official_stable` -> `--client-channel stable`
- `official_preview` -> `--client-channel preview --enable-preview`
- `official_legacy` -> `--legacy-auth-flow`
- `auto` -> 不追加参数

### Codex

- `official_stable` -> `--channel stable`
- `official_preview` -> `--channel preview --enable-beta`
- `official_legacy` -> `--legacy-auth-flow`
- `auto` -> 不追加参数

### Generic

- `official_legacy` -> `--legacy-auth-flow`
- 其他模式 -> 自动回退到 `auto`

## 3. 客户端版本 -> 本地网关行为映射

| 模式 | gateway_mode | gateway_transport | gateway_routing |
| --- | --- | --- | --- |
| `official_stable` | `strict` | `oauth_refresh`(Gemini) / `oauth_token`(Codex) | `${platform}_official` |
| `official_preview` | `compat_preview` | `oauth_refresh` / `oauth_token` | `${platform}_preview` |
| `official_legacy` | `legacy_compat` | `oauth_legacy` | `${platform}_legacy` |
| `auto` | `auto` | `auto` | `auto` |

说明：Wakeup 通过命令模板占位符将映射结果注入执行命令，不额外引入第二套网关状态存储。

## 4. 回退策略

1. 先尝试 `client_version_mode`
2. 若当前平台不支持，尝试 `client_version_fallback_mode`
3. 若仍不支持，强制回退 `auto`

执行结果中会写入“请求模式/生效模式/回退原因”，用于历史追溯。

## 5. 任务模板占位符

- `{client_version_mode}` 生效模式
- `{client_version_mode_requested}` 请求模式
- `{client_version_fallback_mode}` 回退模式
- `{client_runtime_args}` 运行参数
- `{gateway_mode}` 网关模式
- `{gateway_transport}` 网关传输策略
- `{gateway_routing}` 网关路由策略
- `{gateway_version_hint}` 网关版本提示
