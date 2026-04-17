# 更新提醒细粒度策略说明

## 策略项

- `skip_version`：跳过指定版本（命中后不再弹出该版本详情）
- `disable_reminders`：关闭更新提醒（仍可手动检查）
- `silent_reminder_strategy`：
  - `immediate`：每次命中都提醒
  - `daily`：同版本 24 小时内仅提醒一次
  - `weekly`：同版本 7 天内仅提醒一次

## 决策顺序

1. 版本为空 -> 不提醒
2. 命中 `skip_version` -> 不提醒
3. `disable_reminders=true` -> 不提醒
4. `silent_reminder_strategy=immediate` -> 直接提醒
5. 若命中的是新版本（不同于 `last_reminded_version`）-> 直接提醒
6. 否则按 `daily/weekly` 时间窗判断是否延后提醒

## 持久化字段

- `last_check_at`：最近检查时间
- `last_reminded_at`：最近一次提醒时间
- `last_reminded_version`：最近一次提醒的版本

## 迁移策略

- 旧版 `update_settings.json` 缺失新增字段时，加载后自动补默认值并回写文件。
