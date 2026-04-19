import { FileJson, FolderOpen, Globe, HardDrive, History, Loader2 } from "lucide-react";
import type {
  LocalImportOptionView,
  Status,
} from "./addAccountWizardTypes";
import "./AddAccountImportTab.css";

type AddAccountImportTabProps = {
  importing: boolean;
  status: Status;
  currentLocalImportOption: LocalImportOptionView | null;
  onScanLocal: () => void;
  onPickVscdb: () => void;
  onImportLocal: () => void;
  onPickImportFiles: () => void;
  onImportV1: () => void;
};

export function AddAccountImportTab({
  importing,
  status,
  currentLocalImportOption,
  onScanLocal,
  onPickVscdb,
  onImportLocal,
  onPickImportFiles,
  onImportV1,
}: AddAccountImportTabProps) {
  const disabled = importing || status === "success";

  return (
    <div className="wiz-tab-content wiz-import-tab">
      <div className="wiz-import-section">
        <h4 className="wiz-import-section-title">
          <HardDrive size={15} /> 方案 A — 从本机 IDE 自动提取
        </h4>
        <p className="wiz-import-desc">
          自动扫描本机已安装 IDE（VS Code / Cursor / Windsurf / Kiro 等）的账号存储，一键导入。
        </p>
        <button className="wiz-import-btn" onClick={onScanLocal} disabled={disabled}>
          {importing ? <Loader2 size={15} className="spin" /> : <HardDrive size={15} />}
          一键扫描本机 IDE 账号
        </button>

        <button className="wiz-import-btn secondary" onClick={onPickVscdb} disabled={disabled}>
          <FolderOpen size={15} />
          选择 .vscdb 文件导入
        </button>
      </div>

      <div className="wiz-import-divider">或</div>

      {currentLocalImportOption && (
        <>
          <div className="wiz-import-section">
            <h4 className="wiz-import-section-title">
              <Globe size={15} /> {currentLocalImportOption.title}
            </h4>
            <p className="wiz-import-desc">{currentLocalImportOption.description}</p>
            <button className="wiz-import-btn" onClick={onImportLocal} disabled={disabled}>
              <Globe size={15} />
              {currentLocalImportOption.buttonLabel}
            </button>
          </div>

          <div className="wiz-import-divider">或</div>
        </>
      )}

      <div className="wiz-import-section">
        <h4 className="wiz-import-section-title">
          <FileJson size={15} /> 方案 B — 导入文件
        </h4>
        <p className="wiz-import-desc">
          支持选择一个或多个文件，兼容 <code>.json</code>、<code>.vscdb</code>、
          <code>auth.json</code>、<code>oauth_creds.json</code> 等常见导入格式。
        </p>
        <button className="wiz-import-btn" onClick={onPickImportFiles} disabled={disabled}>
          <FileJson size={15} />
          选择文件导入
        </button>
      </div>

      <div className="wiz-import-divider">或</div>

      <div className="wiz-import-section">
        <h4 className="wiz-import-section-title">
          <History size={15} /> 方案 C — 旧版 v1 账号迁移
        </h4>
        <p className="wiz-import-desc">
          从旧版 AI Singularity / Antigravity 的本地存储迁移历史账号数据。
        </p>
        <button className="wiz-import-btn" onClick={onImportV1} disabled={disabled}>
          <History size={15} />
          迁移旧版 v1 账号
        </button>
      </div>
    </div>
  );
}
