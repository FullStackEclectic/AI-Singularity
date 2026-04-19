import type { ImportSummary } from "./addAccountWizardTypes";
import "./AddAccountImportTab.css";

type AddAccountImportSummaryProps = {
  importSummary: ImportSummary | null;
};

export function AddAccountImportSummary({
  importSummary,
}: AddAccountImportSummaryProps) {
  if (!importSummary) {
    return null;
  }

  return (
    <div className="wiz-import-summary">
      <div className="wiz-import-summary-head">
        <strong>导入结果汇总</strong>
        <span>
          成功 {importSummary.ok} / 失败 {importSummary.fail}
        </span>
      </div>
      {importSummary.failures.length > 0 && (
        <div className="wiz-import-summary-list">
          {importSummary.failures.slice(0, 8).map((item, index) => (
            <div
              key={`${item.label}-${index}`}
              className="wiz-import-summary-item failure"
            >
              <div className="wiz-import-summary-title">
                {item.label} · {item.origin_platform}
              </div>
              <div className="wiz-import-summary-meta">{item.source_path}</div>
              <div className="wiz-import-summary-reason">
                {item.reason || "未知错误"}
              </div>
            </div>
          ))}
          {importSummary.failures.length > 8 && (
            <div className="wiz-import-summary-more">
              其余 {importSummary.failures.length - 8} 条失败记录已省略显示。
            </div>
          )}
        </div>
      )}
      {importSummary.ok > 0 && importSummary.fail === 0 && (
        <div className="wiz-import-summary-success">所有账号已成功导入。</div>
      )}
    </div>
  );
}
