import type { AccountGroup } from "../../types";
import type { AccountGroupDialogState } from "./unifiedAccountsTypes";
import "./UnifiedAccountsGroupDialog.css";

export function UnifiedAccountsGroupDialog({
  accountGroupDialog,
  accountGroupBusy,
  accountGroups,
  newGroupName,
  renamingGroupId,
  renamingGroupName,
  onClose,
  onNewGroupNameChange,
  onCreateGroup,
  onStartRename,
  onRenamingGroupNameChange,
  onSaveRename,
  onCancelRename,
  onDeleteGroup,
  onAssignToGroup,
  onRemoveFromGroup,
}: {
  accountGroupDialog: AccountGroupDialogState | null;
  accountGroupBusy: boolean;
  accountGroups: AccountGroup[];
  newGroupName: string;
  renamingGroupId: string | null;
  renamingGroupName: string;
  onClose: () => void;
  onNewGroupNameChange: (value: string) => void;
  onCreateGroup: () => void;
  onStartRename: (group: AccountGroup) => void;
  onRenamingGroupNameChange: (value: string) => void;
  onSaveRename: (groupId: string) => void;
  onCancelRename: () => void;
  onDeleteGroup: (group: AccountGroup) => void;
  onAssignToGroup: (group: AccountGroup) => void;
  onRemoveFromGroup: (group: AccountGroup) => void;
}) {
  if (!accountGroupDialog) {
    return null;
  }

  return (
    <div className="accounts-modal-overlay" onClick={() => !accountGroupBusy && onClose()}>
      <div className="accounts-modal" onClick={(event) => event.stopPropagation()}>
        <h3 className="accounts-modal-title">
          {accountGroupDialog.mode === "manage" ? "账号分组管理" : "批量账号分组"}
        </h3>
        <p className="accounts-modal-desc">
          {accountGroupDialog.mode === "manage"
            ? "维护 IDE 账号分组，并为后续批量操作与调度筛选提供基础。"
            : `当前将处理 ${accountGroupDialog.channelLabel} 下的 ${accountGroupDialog.count} 个 IDE 账号。`}
        </p>

        <div className="accounts-form-group">
          <label className="accounts-form-label">新建分组</label>
          <div className="accounts-inline-row">
            <input
              className="accounts-form-input"
              placeholder="例如：主力 / 备用 / 待验证"
              value={newGroupName}
              onChange={(event) => onNewGroupNameChange(event.target.value)}
              disabled={accountGroupBusy}
            />
            <button
              className="btn-primary"
              disabled={accountGroupBusy || !newGroupName.trim()}
              onClick={onCreateGroup}
            >
              创建
            </button>
          </div>
        </div>

        <div className="accounts-group-list">
          {accountGroups.length === 0 ? (
            <div className="empty-text">当前还没有账号分组</div>
          ) : (
            accountGroups.map((group) => (
              <div key={group.id} className="accounts-group-item">
                <div className="accounts-group-main">
                  {renamingGroupId === group.id ? (
                    <input
                      className="accounts-form-input"
                      value={renamingGroupName}
                      onChange={(event) => onRenamingGroupNameChange(event.target.value)}
                      disabled={accountGroupBusy}
                    />
                  ) : (
                    <div className="accounts-group-name">{group.name}</div>
                  )}
                  <div className="accounts-group-meta">{group.account_ids.length} 个账号</div>
                </div>
                <div className="accounts-group-actions">
                  {accountGroupDialog.mode === "assign" && (
                    <>
                      <button
                        className="btn-outline"
                        disabled={accountGroupBusy || accountGroupDialog.ids.length === 0}
                        onClick={() => onAssignToGroup(group)}
                      >
                        加入分组
                      </button>
                      <button
                        className="btn-outline"
                        disabled={accountGroupBusy || accountGroupDialog.ids.length === 0}
                        onClick={() => onRemoveFromGroup(group)}
                      >
                        移出分组
                      </button>
                    </>
                  )}
                  {accountGroupDialog.mode === "manage" && (
                    <>
                      {renamingGroupId === group.id ? (
                        <>
                          <button
                            className="btn-outline"
                            disabled={accountGroupBusy || !renamingGroupName.trim()}
                            onClick={() => onSaveRename(group.id)}
                          >
                            保存
                          </button>
                          <button
                            className="btn-outline"
                            disabled={accountGroupBusy}
                            onClick={onCancelRename}
                          >
                            取消
                          </button>
                        </>
                      ) : (
                        <button className="btn-outline" onClick={() => onStartRename(group)}>
                          重命名
                        </button>
                      )}
                      <button
                        className="btn-danger-solid"
                        disabled={accountGroupBusy}
                        onClick={() => onDeleteGroup(group)}
                      >
                        删除
                      </button>
                    </>
                  )}
                </div>
              </div>
            ))
          )}
        </div>

        <div className="accounts-modal-actions">
          <button className="btn-outline" onClick={onClose} disabled={accountGroupBusy}>
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}
