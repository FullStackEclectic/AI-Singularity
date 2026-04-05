import { useState, useEffect } from "react";
import {
  getGroups,
  createGroup,
  renameGroup,
  deleteGroup,
  assignAccountsToGroup,
  removeAccountsFromGroup,
  type AccountGroup,
} from "../../lib/groupService";
import type { IdeAccount, ApiKey } from "../../types";
import "./GroupManagerModal.css";

interface GroupManagerModalProps {
  ideAccounts: IdeAccount[];
  apiKeys: ApiKey[];
  onClose: () => void;
  onGroupsChanged: () => void;
}

export default function GroupManagerModal({
  ideAccounts,
  apiKeys,
  onClose,
  onGroupsChanged,
}: GroupManagerModalProps) {
  const [groups, setGroups] = useState<AccountGroup[]>([]);
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [newGroupName, setNewGroupName] = useState("");
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");

  const refresh = () => {
    const g = getGroups();
    setGroups(g);
    onGroupsChanged();
  };

  useEffect(() => {
    refresh();
  }, []);

  const selectedGroup = groups.find((g) => g.id === selectedGroupId);

  const handleCreateGroup = () => {
    if (!newGroupName.trim()) return;
    const g = createGroup(newGroupName);
    setNewGroupName("");
    setSelectedGroupId(g.id);
    refresh();
  };

  const handleDeleteGroup = (id: string) => {
    if (!window.confirm("确认删除此分组？（账号不会被删除）")) return;
    deleteGroup(id);
    if (selectedGroupId === id) setSelectedGroupId(null);
    refresh();
  };

  const handleRenameStart = (g: AccountGroup) => {
    setRenamingId(g.id);
    setRenameValue(g.name);
  };

  const handleRenameCommit = () => {
    if (!renamingId || !renameValue.trim()) return;
    renameGroup(renamingId, renameValue);
    setRenamingId(null);
    refresh();
  };

  const allAccounts = [
    ...ideAccounts.map((a) => ({ id: a.id, label: a.email, type: "ide" as const, platform: a.origin_platform })),
    ...apiKeys.map((k) => ({ id: k.id, label: k.name, type: "api" as const, platform: k.platform })),
  ];

  const isInGroup = (accountId: string) =>
    selectedGroup?.accountIds.includes(accountId) ?? false;

  const toggleAccount = (accountId: string) => {
    if (!selectedGroupId) return;
    if (isInGroup(accountId)) {
      removeAccountsFromGroup(selectedGroupId, [accountId]);
    } else {
      assignAccountsToGroup(selectedGroupId, [accountId]);
    }
    refresh();
  };

  return (
    <div className="gm-overlay" onClick={(e) => e.target === e.currentTarget && onClose()}>
      <div className="gm-modal">
        {/* Header */}
        <div className="gm-header">
          <div>
            <h2 className="gm-title">账号分组管理</h2>
            <p className="gm-subtitle">创建分组，将账号归类便于管理和过滤</p>
          </div>
          <button className="gm-close" onClick={onClose}>✕</button>
        </div>

        <div className="gm-body">
          {/* Left: Group List */}
          <div className="gm-sidebar">
            <div className="gm-sidebar-header">分组列表</div>

            {groups.length === 0 && (
              <div className="gm-empty-groups">暂无分组，在下方创建</div>
            )}

            {groups.map((g) => (
              <div
                key={g.id}
                className={`gm-group-item ${selectedGroupId === g.id ? "active" : ""}`}
                onClick={() => setSelectedGroupId(g.id)}
              >
                {renamingId === g.id ? (
                  <input
                    className="gm-rename-input"
                    value={renameValue}
                    onChange={(e) => setRenameValue(e.target.value)}
                    onBlur={handleRenameCommit}
                    onKeyDown={(e) => e.key === "Enter" && handleRenameCommit()}
                    autoFocus
                    onClick={(e) => e.stopPropagation()}
                  />
                ) : (
                  <>
                    <span className="gm-group-name">{g.name}</span>
                    <span className="gm-group-count">{g.accountIds.length}</span>
                    <div className="gm-group-actions">
                      <button
                        className="gm-icon-btn"
                        onClick={(e) => { e.stopPropagation(); handleRenameStart(g); }}
                        title="重命名"
                      >✎</button>
                      <button
                        className="gm-icon-btn danger"
                        onClick={(e) => { e.stopPropagation(); handleDeleteGroup(g.id); }}
                        title="删除分组"
                      >✕</button>
                    </div>
                  </>
                )}
              </div>
            ))}

            {/* Create new group */}
            <div className="gm-create-row">
              <input
                className="gm-create-input"
                value={newGroupName}
                onChange={(e) => setNewGroupName(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleCreateGroup()}
                placeholder="新建分组名称..."
              />
              <button
                className="gm-create-btn"
                onClick={handleCreateGroup}
                disabled={!newGroupName.trim()}
              >
                ＋
              </button>
            </div>
          </div>

          {/* Right: Account Picker */}
          <div className="gm-content">
            {!selectedGroup ? (
              <div className="gm-pick-hint">
                <div className="gm-pick-icon">👈</div>
                <div>选择左侧分组，然后在此管理成员</div>
              </div>
            ) : (
              <>
                <div className="gm-content-header">
                  <span>「{selectedGroup.name}」的成员（{selectedGroup.accountIds.length}/{allAccounts.length}）</span>
                </div>
                <div className="gm-account-list">
                  {allAccounts.map((acc) => {
                    const inGroup = isInGroup(acc.id);
                    return (
                      <div
                        key={acc.id}
                        className={`gm-acc-item ${inGroup ? "in-group" : ""}`}
                        onClick={() => toggleAccount(acc.id)}
                      >
                        <div className={`gm-acc-check ${inGroup ? "checked" : ""}`}>
                          {inGroup && "✓"}
                        </div>
                        <div className="gm-acc-info">
                          <div className="gm-acc-label">{acc.label}</div>
                          <div className="gm-acc-platform">
                            <span className={`gm-acc-type ${acc.type}`}>
                              {acc.type === "ide" ? "IDE" : "API"}
                            </span>
                            {" "}{acc.platform}
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </>
            )}
          </div>
        </div>

        <div className="gm-footer">
          <button className="gm-btn-primary" onClick={onClose}>完成</button>
        </div>
      </div>
    </div>
  );
}
