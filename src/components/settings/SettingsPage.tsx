import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";

import { check } from "@tauri-apps/plugin-updater";

export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");
  const [updateMsg, setUpdateMsg] = useState("");
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);

  const handleLanguageChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const lang = e.target.value;
    i18n.changeLanguage(lang);
    localStorage.setItem("ais_lang", lang);
  };

  const handleCheckUpdate = async () => {
    setIsCheckingUpdate(true);
    setUpdateMsg("正在请求更新服务器...");
    try {
      const update = await check();
      if (update) {
        setUpdateMsg(`发现新版本 ${update.version}！正在后台下载并安装...`);
        let downloaded = 0;
        let contentLength = 0;
        // alternatively we can just use wait
        await update.downloadAndInstall((event) => {
          switch (event.event) {
            case 'Started':
              contentLength = event.data.contentLength || 0;
              setUpdateMsg(`开始下载新版本：${contentLength} bytes`);
              break;
            case 'Progress':
              downloaded += event.data.chunkLength;
              setUpdateMsg(`已下载 ${downloaded} / ${contentLength}`);
              break;
            case 'Finished':
              setUpdateMsg('下载完成，请重启应用');
              break;
          }
        });
        setUpdateMsg("更新已安装！需要重启应用后生效。");
      } else {
        setUpdateMsg("当前已是最新版本");
      }
    } catch (e) {
      setUpdateMsg(`检查更新失败: ${String(e)} (可能是因为测试地址或网络问题)`);
    } finally {
      setIsCheckingUpdate(false);
    }
  };

  const handleExport = async () => {
    try {
      setLoading(true);
      setMessage("正在导出配置...");
      const data = await invoke("export_config");
      const jsonStr = JSON.stringify(data, null, 2);
      
      const blob = new Blob([jsonStr], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `ai-singularity-config-${new Date().toISOString().replace(/[:.]/g, "-")}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      
      setMessage("配置导出成功！");
    } catch (e) {
      setMessage(`导出失败: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleImport = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    try {
      setLoading(true);
      setMessage("正在读取文件...");
      const text = await file.text();
      
      setMessage("正在导入配置...");
      await invoke("import_config", { jsonData: text });
      setMessage("配置导入成功！后台数据已刷新。");
      // 可以在此处提示重启，或依靠前端其他机制拉取最新状态
    } catch (err) {
      setMessage(`导入失败: ${err}`);
    } finally {
      setLoading(false);
      // reset input
      e.target.value = "";
    }
  };

  return (
    <div>
      <div className="page-header">
        <div>
          <h1 className="page-title">{t("settings.title")}</h1>
          <p className="page-subtitle">{t("settings.subtitle")}</p>
        </div>
      </div>

      <div className="settings-section" style={{ padding: "var(--space-6)" }}>
        <h3 style={{ marginBottom: "var(--space-2)" }}>{t("settings.language")}</h3>
        <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>{t("settings.language_desc")}</p>
        <div style={{ marginBottom: "var(--space-6)" }}>
          <select 
            className="form-input" 
            style={{ width: "200px" }}
            value={i18n.language.startsWith("zh") ? "zh" : "en"}
            onChange={handleLanguageChange}
          >
            <option value="zh">简体中文</option>
            <option value="en">English</option>
          </select>
        </div>

        <h3 style={{ marginBottom: "var(--space-4)" }}>配置与备份</h3>
        <p style={{ color: "var(--color-text-secondary)", marginBottom: "var(--space-4)" }}>
          将所有的 Provider、API Key、MCP 以及 Prompt 导出为一个独立文件，方便多端同步或备份。
        </p>

        <div style={{ display: "flex", gap: "var(--space-4)", marginBottom: "var(--space-6)" }}>
          <button 
            className="btn btn-primary" 
            onClick={handleExport}
            disabled={loading}
          >
            导出配置
          </button>
          
          <label className="btn btn-secondary" style={{ cursor: loading ? "not-allowed" : "pointer" }}>
            导入配置
            <input 
              type="file" 
              accept=".json" 
              style={{ display: "none" }} 
              onChange={handleImport}
              disabled={loading}
            />
          </label>
        </div>

        {message && (
          <div style={{ padding: "var(--space-2)", background: "var(--surface-sunken)", borderRadius: "var(--radius-sm)", fontSize: 14, marginBottom: "var(--space-6)" }}>
            {message}
          </div>
        )}

        {/* 自动更新区 */}
        <h3 style={{ marginBottom: "var(--space-2)" }}>{t("settings.auto_update")}</h3>
        <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>{t("settings.auto_update_desc")}</p>
        <div style={{ marginBottom: "var(--space-6)", display: "flex", flexDirection: "column", gap: "var(--space-3)", alignItems: "flex-start" }}>
          <button 
            className="btn btn-primary"
            onClick={handleCheckUpdate}
            disabled={isCheckingUpdate}
          >
            {isCheckingUpdate ? "检查中..." : t("settings.check_now")}
          </button>
          {updateMsg && (
            <div className="alert alert-info" style={{ fontSize: 13, alignSelf: "stretch" }}>
              {updateMsg}
            </div>
          )}
        </div>

      </div>
    </div>
  );
}
