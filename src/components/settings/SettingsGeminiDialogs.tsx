import type { IdeAccount } from "../../types";
import type { GeminiEditDialogState } from "./settingsTypes";

type SettingsGeminiDialogsProps = {
  geminiEditDialog: GeminiEditDialogState | null;
  confirmDeleteGeminiId: string | null;
  geminiAccounts: IdeAccount[];
  currentGeminiAccountId: string | null;
  editEffectiveGeminiAccountId: string | null;
  editSelectedGeminiProjectId: string | null;
  editAccountProjectId: string | null;
  getGeminiAccountLabel: (accountId?: string | null) => string;
  onCloseEdit: () => void;
  onApplyGeminiEditPatch: (patch: Partial<GeminiEditDialogState>) => void;
  onSaveGeminiEdit: () => void;
  onCloseDelete: () => void;
  onConfirmDelete: (id: string) => void;
};

export function SettingsGeminiDialogs({
  geminiEditDialog,
  confirmDeleteGeminiId,
  geminiAccounts,
  currentGeminiAccountId,
  editEffectiveGeminiAccountId,
  editSelectedGeminiProjectId,
  editAccountProjectId,
  getGeminiAccountLabel,
  onCloseEdit,
  onApplyGeminiEditPatch,
  onSaveGeminiEdit,
  onCloseDelete,
  onConfirmDelete,
}: SettingsGeminiDialogsProps) {
  return (
    <>
      {geminiEditDialog ? (
        <div className="modal-overlay" onClick={onCloseEdit}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>设置 Gemini 实例</h2>
              <button className="btn btn-icon" onClick={onCloseEdit}>
                ✕
              </button>
            </div>
            <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <div>
                <label className="form-label">额外启动参数</label>
                <input
                  className="form-input"
                  value={geminiEditDialog.extraArgs}
                  onChange={(e) =>
                    onApplyGeminiEditPatch({ extraArgs: e.target.value })
                  }
                />
              </div>
              <div>
                <label className="form-label">绑定账号 ID</label>
                <select
                  className="form-input"
                  value={geminiEditDialog.bindAccountId}
                  onChange={(e) =>
                    onApplyGeminiEditPatch({ bindAccountId: e.target.value })
                  }
                  disabled={
                    geminiEditDialog.instance.is_default &&
                    geminiEditDialog.followLocalAccount
                  }
                >
                  <option value="">未绑定账号</option>
                  {geminiAccounts.map((account) => (
                    <option key={account.id} value={account.id}>
                      {(account.label?.trim() || account.email)}
                      {currentGeminiAccountId === account.id ? " (当前本地)" : ""}
                    </option>
                  ))}
                </select>
                <div className="text-muted" style={{ fontSize: 12, marginTop: 6 }}>
                  {geminiEditDialog.instance.is_default &&
                  geminiEditDialog.followLocalAccount
                    ? `当前会跟随本地 Gemini 账号：${getGeminiAccountLabel(currentGeminiAccountId)}`
                    : `当前选择：${getGeminiAccountLabel(geminiEditDialog.bindAccountId || null)}`}
                </div>
              </div>
              <div>
                <label className="form-label">项目 ID</label>
                <input
                  className="form-input"
                  value={geminiEditDialog.projectId}
                  onChange={(e) =>
                    onApplyGeminiEditPatch({ projectId: e.target.value })
                  }
                />
                <div className="text-muted" style={{ fontSize: 12, marginTop: 6 }}>
                  {editSelectedGeminiProjectId
                    ? `实例会固定使用项目 ${editSelectedGeminiProjectId}`
                    : editAccountProjectId
                      ? `当前会沿用账号默认项目 ${editAccountProjectId}`
                      : "当前没有项目覆盖，会直接沿用本地 Gemini 默认行为"}
                </div>
              </div>
              {geminiEditDialog.instance.is_default ? (
                <label style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 13 }}>
                  <input
                    type="checkbox"
                    checked={geminiEditDialog.followLocalAccount}
                    onChange={(e) =>
                      onApplyGeminiEditPatch({
                        followLocalAccount: e.target.checked,
                      })
                    }
                  />
                  跟随当前本地 Gemini 账号
                </label>
              ) : null}
              {geminiEditDialog.instance.is_default &&
              geminiEditDialog.followLocalAccount &&
              !currentGeminiAccountId ? (
                <div className="alert alert-info" style={{ fontSize: 13 }}>
                  当前没有解析到本地 Gemini 账号。若继续保持跟随模式，默认实例启动时不会有可注入的账号。
                </div>
              ) : null}
              {!geminiEditDialog.instance.is_default ||
              !geminiEditDialog.followLocalAccount ? (
                editEffectiveGeminiAccountId &&
                currentGeminiAccountId &&
                editEffectiveGeminiAccountId !== currentGeminiAccountId ? (
                  <div className="alert alert-info" style={{ fontSize: 13 }}>
                    这个实例启动时会把当前本地账号从 {getGeminiAccountLabel(currentGeminiAccountId)} 切换为 {getGeminiAccountLabel(editEffectiveGeminiAccountId)}。
                  </div>
                ) : null
              ) : null}
              {editSelectedGeminiProjectId &&
              editAccountProjectId &&
              editSelectedGeminiProjectId !== editAccountProjectId ? (
                <div className="alert alert-info" style={{ fontSize: 13 }}>
                  实例项目 {editSelectedGeminiProjectId} 会覆盖账号默认项目 {editAccountProjectId}。
                </div>
              ) : null}
              <div
                style={{
                  padding: "10px 12px",
                  borderRadius: "var(--radius-sm)",
                  background: "rgba(255,255,255,0.04)",
                  display: "flex",
                  flexDirection: "column",
                  gap: 8,
                }}
              >
                <div style={{ fontSize: 12, fontWeight: 600 }}>快捷修正建议</div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  {geminiEditDialog.instance.is_default &&
                  currentGeminiAccountId &&
                  !geminiEditDialog.followLocalAccount ? (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() =>
                        onApplyGeminiEditPatch({
                          followLocalAccount: true,
                          bindAccountId: "",
                        })
                      }
                    >
                      改为跟随当前本地账号
                    </button>
                  ) : null}
                  {!geminiEditDialog.followLocalAccount &&
                  currentGeminiAccountId &&
                  geminiEditDialog.bindAccountId !== currentGeminiAccountId ? (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() =>
                        onApplyGeminiEditPatch({
                          bindAccountId: currentGeminiAccountId,
                        })
                      }
                    >
                      改为绑定当前本地账号
                    </button>
                  ) : null}
                  {!geminiEditDialog.followLocalAccount &&
                  !geminiEditDialog.bindAccountId &&
                  currentGeminiAccountId ? (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() =>
                        onApplyGeminiEditPatch({
                          bindAccountId: currentGeminiAccountId,
                        })
                      }
                    >
                      绑定当前本地账号
                    </button>
                  ) : null}
                  {editSelectedGeminiProjectId ? (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() => onApplyGeminiEditPatch({ projectId: "" })}
                    >
                      清除实例项目覆盖
                    </button>
                  ) : null}
                  {!editSelectedGeminiProjectId && editAccountProjectId ? (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() =>
                        onApplyGeminiEditPatch({ projectId: editAccountProjectId })
                      }
                    >
                      固定为账号默认项目
                    </button>
                  ) : null}
                  {geminiEditDialog.followLocalAccount &&
                  !currentGeminiAccountId &&
                  geminiAccounts.length > 0 ? (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={() =>
                        onApplyGeminiEditPatch({
                          followLocalAccount: false,
                          bindAccountId: geminiAccounts[0]?.id || "",
                        })
                      }
                    >
                      改为固定绑定账号
                    </button>
                  ) : null}
                </div>
                <div className="text-muted" style={{ fontSize: 12 }}>
                  这些操作只会改当前弹层里的配置草稿，真正生效仍然要点保存。
                </div>
              </div>
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={onCloseEdit}>
                  取消
                </button>
                <button className="btn btn-primary" onClick={onSaveGeminiEdit}>
                  保存
                </button>
              </div>
            </div>
          </div>
        </div>
      ) : null}

      {confirmDeleteGeminiId ? (
        <div className="modal-overlay" onClick={onCloseDelete}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>删除 Gemini 实例</h2>
              <button className="btn btn-icon" onClick={onCloseDelete}>
                ✕
              </button>
            </div>
            <div className="modal-body">
              <p>确认删除这个 Gemini 实例目录索引吗？不会删除真实文件。</p>
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={onCloseDelete}>
                  取消
                </button>
                <button
                  className="btn btn-danger"
                  onClick={() => onConfirmDelete(confirmDeleteGeminiId)}
                >
                  删除
                </button>
              </div>
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
