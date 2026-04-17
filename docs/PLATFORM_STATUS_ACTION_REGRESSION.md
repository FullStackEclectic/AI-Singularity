# 平台状态动作回归清单（daily_checkin）

更新时间：2026-04-16

## 覆盖范围

- 后端平台状态动作服务：`daily_checkin`
- 适配平台：`codebuddy_cn`、`workbuddy`
- 前端入口：
  - 统一资产页头部“一键签到”
  - 列表行“签到”按钮
  - 网格卡片“签到”按钮

## 预置数据

- 至少 1 个 `codebuddy_cn` 账号，且 `token.access_token` 有效。
- 至少 1 个 `workbuddy` 账号，且 `token.access_token` 有效。
- 至少 1 个非支持签到平台账号（如 `cursor`）用于负向验证。

## 手工回归步骤

1. 打开“统一资产页”，切到包含 `codebuddy_cn/workbuddy` 的视图。
2. 点击单账号签到按钮，确认页面出现执行结果提示。
3. 对已签到账号重复点击，确认展示“未完成/已签到”等可理解反馈，而不是前端报错。
4. 在有多个可签到账号时点击头部“一键签到”，确认出现“成功/未完成/失败”汇总。
5. 选中部分账号后再次执行“一键签到”，确认只处理已选账号。
6. 切换到不支持签到的平台视图，确认不显示签到快捷动作。
7. 触发一次失败场景（无效 token 或断网），确认提示中包含失败原因，且不会影响页面其他操作。
8. 执行成功后刷新页面，确认账号仍可正常显示，且没有破坏既有 `meta_json` 字段。

## 数据检查（可选）

- 在数据库中抽查对应 `ide_accounts.meta_json`：
  - 应存在 `status_actions.daily_checkin` 节点
  - 节点应包含：
    - `last_attempt_at`
    - `success`
    - `message`
    - `reward`（可为空）
    - `next_checkin_in`（可为空）

## 自动化检查

- Rust 单测：
  - `services::platform_status_action::tests::pick_string_recursive_finds_nested_values`
  - `services::platform_status_action::tests::extract_auth_context_handles_invalid_meta`
  - `services::platform_status_action::tests::merge_status_action_meta_inserts_payload_and_preserves_existing_fields`
  - `services::platform_status_action::tests::merge_status_action_meta_recovers_from_non_object_meta`

运行命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml platform_status_action -- --nocapture
```

- 前端工具单测：
  - `platformStatusActionUtils.test.ts`
  - `accountViewUtils.test.ts`

运行命令：

```powershell
node .\node_modules\vitest\vitest.mjs run src\components\accounts\platformStatusActionUtils.test.ts src\components\accounts\accountViewUtils.test.ts
```
