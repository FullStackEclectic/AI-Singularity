# 模块化审计清单（2026-04-16）

## 扫描范围

- 前端：`src/**`（`.ts` / `.tsx` / `.css`）
- 后端：`src-tauri/src/**`（`.rs`）
- 排除：`references/`、`node_modules/`、`target/`

## 规模统计

- `>= 1200` 行文件：`9`
- `>= 800` 行文件：`13`
- `>= 500` 行文件：`24`
- `>= 300` 行文件：`47`

按扩展名：

- `.tsx`：46 个，平均 378 行，最大 2667 行
- `.rs`：103 个，平均 239 行，最大 2035 行
- `.css`：28 个，平均 243 行，最大 816 行
- `.ts`：24 个，平均 102 行，最大 542 行

## P0（立即拆分，超大且高复杂）

1. `src/components/accounts/UnifiedAccountsList.tsx`（2667）
2. `src-tauri/src/services/session_manager.rs`（2035）
3. `src/components/settings/SettingsPage.tsx`（1766）
4. `src-tauri/src/services/ide_injector.rs`（1683）
5. `src-tauri/src/services/oauth.rs`（1652）
6. `src/components/sessions/SessionsPage.tsx`（1601）
7. `src-tauri/src/services/ide_scanner.rs`（1423）
8. `src/components/accounts/AddAccountWizard.tsx`（1383）
9. `src-tauri/src/services/wakeup.rs`（1360）

## P1（高优先，体量大/耦合高）

1. `src/components/wakeup/WakeupPage.tsx`（1117）
2. `src-tauri/src/proxy/server.rs`（1038）
3. `src/components/providers/ProviderModal.tsx`（831）
4. `src/components/sessions/SessionsPage.css`（816）
5. `src/components/providers/ProviderModal.css`（761）
6. `src-tauri/src/services/provider_current.rs`（748）
7. `src/components/accounts/AddAccountWizard.css`（713）
8. `src/components/accounts/UnifiedAccountsList.css`（706）
9. `src-tauri/src/services/codex_ide.rs`（678）
10. `src-tauri/src/services/update_manager.rs`（652）
11. `src-tauri/src/db.rs`（639）
12. `src-tauri/src/models.rs`（622）
13. `src-tauri/src/tray.rs`（564）
14. `src/lib/api.ts`（542）
15. `src/components/analytics/AnalyticsPage.tsx`（524）

## 前端复杂度热点（hooks 聚合）

1. `src/components/sessions/SessionsPage.tsx`：hooks 40（`useMemo` 28）
2. `src/components/wakeup/WakeupPage.tsx`：hooks 28
3. `src/components/accounts/UnifiedAccountsList.tsx`：hooks 25
4. `src/components/settings/SettingsPage.tsx`：hooks 18
5. `src/components/accounts/AddAccountWizard.tsx`：hooks 16

## Rust 复杂度热点（函数数聚合）

1. `src-tauri/src/services/ide_injector.rs`：59
2. `src-tauri/src/services/wakeup.rs`：53
3. `src-tauri/src/services/session_manager.rs`：51
4. `src-tauri/src/services/ide_scanner.rs`：50
5. `src-tauri/src/services/oauth.rs`：32
6. `src-tauri/src/services/provider_current.rs`：32

## 拆分建议模板

### 前端大页面（TSX）

- 页面容器：仅保留数据流编排与路由状态。
- `hooks/`：按领域拆为 `useXxxQuery` / `useXxxActions` / `useXxxFilters`。
- `components/`：视图块拆为可独立测试组件（header、filters、table/grid、dialogs）。
- `utils/`：格式化、条件判断、映射逻辑下沉。
- `types/`：页面私有类型移出主文件。

### Rust 大服务（RS）

- `service orchestration` 与 `provider adapter` 分离。
- I/O（HTTP/FS/DB）与纯逻辑（解析/决策）分层。
- 解析器/规则引擎独立 `mod parser` / `mod policy`。
- 大量 `match platform` 逻辑拆为 `platform/*` 子模块。
- 先为纯逻辑补单测，再迁移 I/O 调用。

## 推荐执行顺序（风险最小）

1. `UnifiedAccountsList.tsx`
2. `SessionsPage.tsx`
3. `SettingsPage.tsx`
4. `AddAccountWizard.tsx`
5. `wakeup.rs`
6. `ide_injector.rs`
7. `ide_scanner.rs`
8. `oauth.rs`
9. `session_manager.rs`

## 进行中（已启动）

- `UnifiedAccountsList.tsx` 已完成第一轮拆分：
  - 抽离纯函数到 `unifiedAccountsUtils.ts`
  - 抽离状态渲染到 `UnifiedAccountStatusCell.tsx`
  - 抽离余额/配额渲染到 `UnifiedAccountBalanceCell.tsx`
  - 主文件行数：`2787 -> 2152`
- `UnifiedAccountsList.tsx` 已完成第二轮拆分：
  - 抽离通用弹窗与编辑弹窗到 `UnifiedAccountsModals.tsx`
  - 主文件继续降至：`2152 -> 2006`
