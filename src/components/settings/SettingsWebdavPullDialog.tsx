type SettingsWebdavPullDialogProps = {
  open: boolean;
  webdavLoading: boolean;
  onClose: () => void;
  onConfirm: () => void;
};

export function SettingsWebdavPullDialog({
  open,
  webdavLoading,
  onClose,
  onConfirm,
}: SettingsWebdavPullDialogProps) {
  if (!open) {
    return null;
  }

  return (
    <div className="modal-overlay" onClick={() => !webdavLoading && onClose()}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>确认 WebDAV Pull</h2>
          <button className="btn btn-icon" onClick={onClose}>
            ✕
          </button>
        </div>
        <div className="modal-body">
          <p>警告：拉取将会用云端配置覆盖本地配置（增量覆盖），确定要继续吗？</p>
          <div className="modal-footer">
            <button className="btn btn-ghost" onClick={onClose} disabled={webdavLoading}>
              取消
            </button>
            <button className="btn btn-danger" onClick={onConfirm} disabled={webdavLoading}>
              {webdavLoading ? "拉取中..." : "确认拉取"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
