import type { ChangeEvent } from "react";
import type { Update as TauriUpdate } from "@tauri-apps/plugin-updater";
import type {
  LinuxReleaseInfo,
  UpdateRuntimeInfo,
  UpdateSettings,
} from "../../lib/api";
import type { UpdateProgressState } from "./settingsTypes";

type SettingsBackupAndUpdateSectionProps = {
  autoUpdateTitle: string;
  autoUpdateDescription: string;
  checkNowLabel: string;
  loading: boolean;
  message: string;
  webdavUrl: string;
  webdavUser: string;
  webdavPass: string;
  webdavMsg: string;
  webdavLoading: boolean;
  updateSettings: UpdateSettings | null;
  updateRuntimeInfo: UpdateRuntimeInfo | null;
  linuxReleaseInfo: LinuxReleaseInfo | null;
  linuxInstallBusyUrl: string | null;
  availableUpdate: TauriUpdate | null;
  isCheckingUpdate: boolean;
  updateMsg: string;
  updateProgress: UpdateProgressState;
  selectedReminderStrategy: string;
  onExport: () => void;
  onImport: (event: ChangeEvent<HTMLInputElement>) => void;
  onWebdavUrlChange: (value: string) => void;
  onWebdavUserChange: (value: string) => void;
  onWebdavPassChange: (value: string) => void;
  onTestWebdav: () => void;
  onPushWebdav: () => void;
  onOpenWebdavPullConfirm: () => void;
  onUpdateSettingChange: (
    patch: Partial<UpdateSettings>,
    successMessage?: string
  ) => void;
  onSkipFoundVersion: () => void;
  onClearSkipVersion: () => void;
  onOpenAssetUrl: (url: string) => void;
  onInstallLinuxAsset: (url: string, kind: string, version: string) => void;
  onCheckUpdate: () => void;
  onInstallUpdate: () => void;
  onCollapseUpdateDetails: () => void;
};

export function SettingsBackupAndUpdateSection({
  autoUpdateTitle,
  autoUpdateDescription,
  checkNowLabel,
  loading,
  message,
  webdavUrl,
  webdavUser,
  webdavPass,
  webdavMsg,
  webdavLoading,
  updateSettings,
  updateRuntimeInfo,
  linuxReleaseInfo,
  linuxInstallBusyUrl,
  availableUpdate,
  isCheckingUpdate,
  updateMsg,
  updateProgress,
  selectedReminderStrategy,
  onExport,
  onImport,
  onWebdavUrlChange,
  onWebdavUserChange,
  onWebdavPassChange,
  onTestWebdav,
  onPushWebdav,
  onOpenWebdavPullConfirm,
  onUpdateSettingChange,
  onSkipFoundVersion,
  onClearSkipVersion,
  onOpenAssetUrl,
  onInstallLinuxAsset,
  onCheckUpdate,
  onInstallUpdate,
  onCollapseUpdateDetails,
}: SettingsBackupAndUpdateSectionProps) {
  return (
    <>
      <h3 style={{ marginBottom: "var(--space-4)" }}>配置与备份</h3>
      <p style={{ color: "var(--color-text-secondary)", marginBottom: "var(--space-4)" }}>
        将所有的 Provider、API Key、MCP 以及 Prompt 导出为一个独立文件，方便多端同步或备份。
      </p>

      <div style={{ display: "flex", gap: "var(--space-4)", marginBottom: "var(--space-6)" }}>
        <button className="btn btn-primary" onClick={onExport} disabled={loading}>
          导出配置
        </button>

        <label
          className="btn btn-secondary"
          style={{ cursor: loading ? "not-allowed" : "pointer" }}
        >
          导入配置
          <input
            type="file"
            accept=".json"
            style={{ display: "none" }}
            onChange={onImport}
            disabled={loading}
          />
        </label>
      </div>

      {message ? (
        <div
          style={{
            padding: "var(--space-2)",
            background: "var(--surface-sunken)",
            borderRadius: "var(--radius-sm)",
            fontSize: 14,
            marginBottom: "var(--space-6)",
          }}
        >
          {message}
        </div>
      ) : null}

      <h3 style={{ marginBottom: "var(--space-2)" }}>多端 WebDAV 备份同步</h3>
      <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
        保障您的配置、Prompt、工具与资产跨端实时同步，数据安全不丢失（原生只支持基于 HTTP 基本认证的 WebDAV）。
      </p>
      <div
        style={{
          background: "var(--surface-sunken)",
          padding: "var(--space-4)",
          borderRadius: "var(--radius-md)",
          marginBottom: "var(--space-6)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--space-3)",
        }}
      >
        <div>
          <label style={{ display: "block", marginBottom: "4px", fontSize: "13px" }}>
            WebDAV 服务器地址 (URL)
          </label>
          <input
            type="text"
            className="form-input"
            placeholder="https://dav.your-server.com/"
            value={webdavUrl}
            onChange={(e) => onWebdavUrlChange(e.target.value)}
          />
        </div>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr",
            gap: "var(--space-3)",
          }}
        >
          <div>
            <label style={{ display: "block", marginBottom: "4px", fontSize: "13px" }}>
              用户名
            </label>
            <input
              type="text"
              className="form-input"
              value={webdavUser}
              onChange={(e) => onWebdavUserChange(e.target.value)}
            />
          </div>
          <div>
            <label style={{ display: "block", marginBottom: "4px", fontSize: "13px" }}>
              密码 / API Token
            </label>
            <input
              type="password"
              className="form-input"
              value={webdavPass}
              onChange={(e) => onWebdavPassChange(e.target.value)}
            />
          </div>
        </div>
        <div style={{ display: "flex", gap: "var(--space-3)", marginTop: "var(--space-2)" }}>
          <button
            className="btn btn-secondary"
            onClick={onTestWebdav}
            disabled={webdavLoading || !webdavUrl}
          >
            测试连接
          </button>
          <button
            className="btn btn-primary"
            onClick={onPushWebdav}
            disabled={webdavLoading || !webdavUrl}
          >
            立即上传推送 (Push)
          </button>
          <button
            className="btn btn-danger"
            onClick={onOpenWebdavPullConfirm}
            disabled={webdavLoading || !webdavUrl}
          >
            立即下载覆盖 (Pull)
          </button>
        </div>
        {webdavMsg ? (
          <div
            style={{
              padding: "var(--space-2)",
              background: "rgba(0,0,0,0.2)",
              borderRadius: "var(--radius-sm)",
              fontSize: 13,
              marginTop: "var(--space-2)",
            }}
          >
            {webdavMsg}
          </div>
        ) : null}
      </div>

      <h3 style={{ marginBottom: "var(--space-2)" }}>{autoUpdateTitle}</h3>
      <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
        {autoUpdateDescription}
      </p>
      <div style={{ marginBottom: "var(--space-6)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
        <div
          style={{
            background: "var(--surface-sunken)",
            padding: "var(--space-4)",
            borderRadius: "var(--radius-md)",
            display: "grid",
            gridTemplateColumns: "1fr 1fr 1fr",
            gap: "var(--space-3)",
          }}
        >
          <div>
            <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>
              当前版本
            </div>
            <div style={{ fontWeight: 600 }}>{updateRuntimeInfo?.current_version || "—"}</div>
          </div>
          <div>
            <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>
              平台
            </div>
            <div style={{ fontWeight: 600 }}>
              {updateRuntimeInfo?.platform || "—"}
              {updateRuntimeInfo?.linux_install_kind
                ? ` · ${updateRuntimeInfo.linux_install_kind}`
                : ""}
            </div>
          </div>
          <div>
            <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>
              上次检查
            </div>
            <div style={{ fontWeight: 600 }}>
              {updateSettings?.last_check_at
                ? new Date(updateSettings.last_check_at).toLocaleString()
                : "尚未检查"}
            </div>
          </div>
        </div>

        <div
          style={{
            background: "var(--surface-sunken)",
            padding: "var(--space-4)",
            borderRadius: "var(--radius-md)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--space-3)",
          }}
        >
          <label style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 13 }}>
            <input
              type="checkbox"
              checked={!!updateSettings?.auto_check}
              onChange={(e) =>
                onUpdateSettingChange(
                  { auto_check: e.target.checked },
                  "更新设置已保存"
                )
              }
            />
            自动检查更新
          </label>
          <label
            style={{
              display: "flex",
              alignItems: "center",
              gap: 10,
              fontSize: 13,
              opacity: updateRuntimeInfo?.can_auto_install === false ? 0.65 : 1,
            }}
          >
            <input
              type="checkbox"
              checked={!!updateSettings?.auto_install}
              disabled={updateRuntimeInfo?.can_auto_install === false}
              onChange={(e) =>
                onUpdateSettingChange(
                  { auto_install: e.target.checked },
                  "更新设置已保存"
                )
              }
            />
            自动安装更新
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 13 }}>
            <input
              type="checkbox"
              checked={!!updateSettings?.disable_reminders}
              onChange={(e) =>
                onUpdateSettingChange(
                  { disable_reminders: e.target.checked },
                  e.target.checked ? "已关闭更新提醒" : "已恢复更新提醒"
                )
              }
            />
            关闭更新提醒（仅手动检查时显示结果）
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 6, fontSize: 13 }}>
            <span>静默提醒策略</span>
            <select
              className="form-input"
              value={selectedReminderStrategy}
              onChange={(e) =>
                onUpdateSettingChange(
                  { silent_reminder_strategy: e.target.value },
                  "静默提醒策略已保存"
                )
              }
              disabled={!!updateSettings?.disable_reminders}
            >
              <option value="immediate">即时提醒（每次命中都提示）</option>
              <option value="daily">每日一次（同版本 24 小时内不重复提醒）</option>
              <option value="weekly">每周一次（同版本 7 天内不重复提醒）</option>
            </select>
          </label>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "1fr auto auto",
              gap: 8,
              alignItems: "end",
            }}
          >
            <label style={{ display: "flex", flexDirection: "column", gap: 6, fontSize: 13 }}>
              <span>跳过指定版本</span>
              <input
                className="form-input"
                value={updateSettings?.skip_version || ""}
                placeholder="例如 0.1.12"
                onChange={(e) =>
                  onUpdateSettingChange(
                    { skip_version: e.target.value || null },
                    "跳过版本策略已保存"
                  )
                }
              />
            </label>
            <button
              className="btn btn-secondary"
              onClick={onSkipFoundVersion}
              disabled={!availableUpdate?.version}
            >
              跳过当前发现版本
            </button>
            <button
              className="btn btn-secondary"
              onClick={onClearSkipVersion}
              disabled={!updateSettings?.skip_version}
            >
              清除跳过
            </button>
          </div>
          <div className="text-muted" style={{ fontSize: 12 }}>
            当前策略：
            {updateSettings?.disable_reminders
              ? " 已关闭提醒"
              : ` ${selectedReminderStrategy === "weekly" ? "每周一次" : selectedReminderStrategy === "daily" ? "每日一次" : "即时提醒"}`}
            {updateSettings?.skip_version
              ? ` · 已跳过 ${updateSettings.skip_version}`
              : " · 未设置跳过版本"}
          </div>
          {updateRuntimeInfo?.warning ? (
            <div className="alert alert-info" style={{ fontSize: 13 }}>
              {updateRuntimeInfo.warning}
            </div>
          ) : null}
          {updateRuntimeInfo?.linux_manual_hint ? (
            <div
              style={{
                padding: "var(--space-3)",
                background: "rgba(0,0,0,0.18)",
                borderRadius: "var(--radius-sm)",
                fontSize: 13,
                lineHeight: 1.6,
              }}
            >
              <div style={{ fontWeight: 600, marginBottom: 6 }}>Linux 安装处理建议</div>
              <div>{updateRuntimeInfo.linux_manual_hint}</div>
            </div>
          ) : null}
          {updateRuntimeInfo?.platform === "linux" &&
          linuxReleaseInfo &&
          linuxReleaseInfo.assets.length > 0 ? (
            <div
              style={{
                padding: "var(--space-3)",
                background: "rgba(0,0,0,0.18)",
                borderRadius: "var(--radius-sm)",
                display: "flex",
                flexDirection: "column",
                gap: 10,
              }}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  gap: 12,
                  alignItems: "center",
                  flexWrap: "wrap",
                }}
              >
                <div>
                  <div style={{ fontWeight: 600 }}>Linux 发行包资产</div>
                  <div className="text-muted" style={{ fontSize: 12 }}>
                    {linuxReleaseInfo.version}
                    {linuxReleaseInfo.published_at
                      ? ` · ${new Date(linuxReleaseInfo.published_at).toLocaleString()}`
                      : ""}
                  </div>
                </div>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {linuxReleaseInfo.assets.map((asset) => (
                  <div
                    key={asset.url}
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      gap: 12,
                      alignItems: "center",
                      flexWrap: "wrap",
                      padding: "10px 12px",
                      borderRadius: "var(--radius-sm)",
                      border: "1px solid var(--color-border)",
                    }}
                  >
                    <div>
                      <div style={{ fontWeight: 500 }}>
                        {asset.name}
                        {asset.preferred ? " · 推荐" : ""}
                      </div>
                      <div className="text-muted" style={{ fontSize: 12 }}>
                        {asset.kind}
                        {typeof asset.size === "number"
                          ? ` · ${Math.round(asset.size / 1024)} KB`
                          : ""}
                      </div>
                    </div>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                      <button
                        className="btn btn-secondary"
                        onClick={() => onOpenAssetUrl(asset.url)}
                      >
                        下载此安装包
                      </button>
                      <button
                        className="btn btn-primary"
                        disabled={linuxInstallBusyUrl === asset.url}
                        onClick={() =>
                          onInstallLinuxAsset(
                            asset.url,
                            asset.kind,
                            linuxReleaseInfo.version
                          )
                        }
                      >
                        {linuxInstallBusyUrl === asset.url
                          ? "处理中..."
                          : asset.kind === "appimage"
                            ? "下载并准备"
                            : "下载并安装"}
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ) : null}
          <div>
            <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>
              Updater Endpoints
            </div>
            {updateRuntimeInfo?.updater_endpoints?.length ? (
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {updateRuntimeInfo.updater_endpoints.map((endpoint) => (
                  <code key={endpoint} style={{ fontSize: 12, wordBreak: "break-all" }}>
                    {endpoint}
                  </code>
                ))}
              </div>
            ) : (
              <div className="text-muted" style={{ fontSize: 13 }}>
                当前未读取到更新地址
              </div>
            )}
          </div>
        </div>

        <div style={{ display: "flex", gap: "var(--space-3)", alignItems: "center", flexWrap: "wrap" }}>
          <button className="btn btn-primary" onClick={onCheckUpdate} disabled={isCheckingUpdate}>
            {isCheckingUpdate ? "检查中..." : checkNowLabel}
          </button>
          <span className="text-muted" style={{ fontSize: 12 }}>
            {updateRuntimeInfo?.updater_pubkey_configured
              ? "Updater 公钥已配置"
              : "Updater 公钥仍为占位值"}
          </span>
        </div>
        {availableUpdate ? (
          <div
            style={{
              background: "var(--surface-sunken)",
              padding: "var(--space-4)",
              borderRadius: "var(--radius-md)",
              display: "flex",
              flexDirection: "column",
              gap: "var(--space-3)",
            }}
          >
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "1fr 1fr 1fr",
                gap: "var(--space-3)",
              }}
            >
              <div>
                <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>
                  发现版本
                </div>
                <div style={{ fontWeight: 600 }}>{availableUpdate.version}</div>
              </div>
              <div>
                <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>
                  发布日期
                </div>
                <div style={{ fontWeight: 600 }}>
                  {availableUpdate.date
                    ? new Date(availableUpdate.date).toLocaleString()
                    : "未知"}
                </div>
              </div>
              <div>
                <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>
                  安装能力
                </div>
                <div style={{ fontWeight: 600 }}>
                  {updateRuntimeInfo?.can_auto_install
                    ? "支持插件自动安装"
                    : "建议手动处理"}
                </div>
              </div>
            </div>

            {availableUpdate.body ? (
              <div>
                <div className="text-muted" style={{ fontSize: 12, marginBottom: 6 }}>
                  发布说明
                </div>
                <pre
                  style={{
                    margin: 0,
                    whiteSpace: "pre-wrap",
                    fontSize: 12,
                    lineHeight: 1.5,
                    padding: "var(--space-3)",
                    background: "rgba(0,0,0,0.18)",
                    borderRadius: "var(--radius-sm)",
                  }}
                >
                  {availableUpdate.body}
                </pre>
              </div>
            ) : null}

            {updateProgress.phase !== "idle" ? (
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12 }}>
                  <span>
                    {updateProgress.phase === "checking" && "正在检查更新"}
                    {updateProgress.phase === "downloading" && "正在下载更新包"}
                    {updateProgress.phase === "installing" && "正在安装更新"}
                    {updateProgress.phase === "finished" && "更新流程完成"}
                  </span>
                  <span>
                    {updateProgress.total > 0
                      ? `${updateProgress.downloaded} / ${updateProgress.total}`
                      : updateProgress.downloaded > 0
                        ? `${updateProgress.downloaded} bytes`
                        : ""}
                  </span>
                </div>
                <div
                  style={{
                    height: 8,
                    borderRadius: 999,
                    background: "rgba(255,255,255,0.08)",
                    overflow: "hidden",
                  }}
                >
                  <div
                    style={{
                      height: "100%",
                      width:
                        updateProgress.total > 0
                          ? `${Math.min(100, (updateProgress.downloaded / updateProgress.total) * 100)}%`
                          : updateProgress.phase === "finished"
                            ? "100%"
                            : "20%",
                      background:
                        "linear-gradient(90deg, var(--color-primary), var(--color-accent))",
                      transition: "width 180ms ease",
                    }}
                  />
                </div>
              </div>
            ) : null}

            <div style={{ display: "flex", gap: "var(--space-3)", flexWrap: "wrap" }}>
              <button
                className="btn btn-primary"
                disabled={
                  isCheckingUpdate ||
                  updateProgress.phase === "downloading" ||
                  updateProgress.phase === "installing" ||
                  updateRuntimeInfo?.can_auto_install === false
                }
                onClick={onInstallUpdate}
              >
                {updateProgress.phase === "downloading" ||
                updateProgress.phase === "installing"
                  ? "处理中..."
                  : "下载并安装"}
              </button>
              <button className="btn btn-secondary" disabled={isCheckingUpdate} onClick={onSkipFoundVersion}>
                跳过此版本
              </button>
              <button
                className="btn btn-secondary"
                disabled={isCheckingUpdate}
                onClick={onCollapseUpdateDetails}
              >
                收起更新详情
              </button>
            </div>
          </div>
        ) : null}
        {updateMsg ? (
          <div className="alert alert-info" style={{ fontSize: 13, alignSelf: "stretch" }}>
            {updateMsg}
          </div>
        ) : null}
      </div>
    </>
  );
}
