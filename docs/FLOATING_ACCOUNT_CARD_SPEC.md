# Floating Account Card 设计草案

## 1. 目标

- 提供轻量浮窗用于快速查看/切换当前账号，不打断主窗口工作流。
- 支持按实例绑定显示，避免多实例场景下账号上下文混淆。
- 为后续多窗口同步与置顶/定位记忆提供统一状态模型。

## 2. 数据模型

### 2.1 浮窗实体 `FloatingAccountCard`

- `id`: string（稳定 ID）
- `scope`: `"global" | "instance"`
- `instance_id?`: string（`scope=instance` 时必填）
- `title`: string（显示标题）
- `bound_platforms`: string[]（如 `["codex","gemini"]`）
- `window_label?`: string（绑定窗口名）
- `always_on_top`: boolean
- `x`: number
- `y`: number
- `width`: number
- `height`: number
- `collapsed`: boolean
- `visible`: boolean
- `updated_at`: string（RFC3339）

### 2.2 运行时快照 `FloatingAccountCardRuntime`

- `card_id`: string
- `current_accounts`: `{ platform: string; account_id?: string; label?: string }[]`
- `last_switch_at?`: string
- `switch_hint?`: string
- `warning?`: string（冲突/失效提示）

## 3. 交互模型

- 创建：
  - 从会话页实例卡片创建“实例绑定浮窗”。
  - 从设置页创建“全局浮窗”。
- 展示：
  - 默认显示当前账号、最近切换时间、快捷操作（复制账号标识、跳转资产页）。
- 操作：
  - 快速切号（调用现有设为当前链路）。
  - 切换置顶。
  - 折叠/展开。
  - 拖拽移动与尺寸记忆。
- 错误态：
  - 账号失效、跨平台不可切换、绑定实例不存在时给出 warning。

## 4. 持久化建议

- 文件：`app_data/floating_account_cards.json`
- 策略：
  - 写入防抖（300ms）。
  - 启动时恢复所有 `visible=true` 的浮窗。
  - 当绑定实例删除时，自动降级为 `scope=global` 并提示。

## 5. 事件总线

- `floating.card.created`
- `floating.card.updated`
- `floating.card.deleted`
- `floating.card.visibility_changed`
- `floating.card.position_changed`
- `floating.account.changed`（用于多窗口同步）

## 6. 与现有模块的边界

- 账号真实切换仍由现有 `force_inject_ide` / current snapshot 链路负责。
- 浮窗仅承担“快捷入口 + 状态展示”，不引入第二套账号模型。
