import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";

import { check } from "@tauri-apps/plugin-updater";
import { api } from "../../lib/api";

export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");
  const [updateMsg, setUpdateMsg] = useState("");
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);

  const [webdavUrl, setWebdavUrl] = useState(localStorage.getItem("webdav_url") || "");
  const [webdavUser, setWebdavUser] = useState(localStorage.getItem("webdav_user") || "");
  const [webdavPass, setWebdavPass] = useState(localStorage.getItem("webdav_pass") || "");
  const [webdavMsg, setWebdavMsg] = useState("");
  const [webdavLoading, setWebdavLoading] = useState(false);

  const saveWebdavConfig = () => {
    localStorage.setItem("webdav_url", webdavUrl);
    localStorage.setItem("webdav_user", webdavUser);
    localStorage.setItem("webdav_pass", webdavPass);
    const config = { url: webdavUrl, username: webdavUser, password: webdavPass || null };
    // Send to backend for daemon usage
    invoke("webdav_save_config", { config }).catch(console.error);
    return config;
  };

  const handleWebdavTest = async () => {
    try {
      setWebdavLoading(true);
      setWebdavMsg("正在测试连接...");
      await api.webdav.testConnection(saveWebdavConfig());
      setWebdavMsg("✅ 测试成功！");
    } catch (e) {
      setWebdavMsg(`❌ 测试失败: ${e}`);
    } finally {
      setWebdavLoading(false);
    }
  };

  const handleWebdavPush = async () => {
    try {
      setWebdavLoading(true);
      setWebdavMsg("正在推送至云端...");
      await api.webdav.push(saveWebdavConfig());
      setWebdavMsg("✅ 推送同步成功！");
    } catch (e) {
      setWebdavMsg(`❌ 推送失败: ${e}`);
    } finally {
      setWebdavLoading(false);
    }
  };

  const handleWebdavPull = async () => {
    if (!window.confirm("警告：拉取将会用云端配置覆盖本地配置（增量覆盖），确定要继续吗？")) return;
    try {
      setWebdavLoading(true);
      setWebdavMsg("正在从云端拉取配置...");
      await api.webdav.pull(saveWebdavConfig());
      setWebdavMsg("✅ 拉取成功！数据已应用。");
    } catch (e) {
      setWebdavMsg(`❌ 拉取失败: ${e}`);
    } finally {
      setWebdavLoading(false);
    }
  };

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

        {/* WebDAV 同步区 */}
        <h3 style={{ marginBottom: "var(--space-2)" }}>多端 WebDAV 备份同步</h3>
        <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
          保障您的配置、Prompt、工具与资产跨端实时同步，数据安全不丢失（原生只支持基于 HTTP 基本认证的 WebDAV）。
        </p>
        <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)", marginBottom: "var(--space-6)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
          <div>
            <label style={{ display: "block", marginBottom: "4px", fontSize: "13px" }}>WebDAV 服务器地址 (URL)</label>
            <input 
              type="text" 
              className="form-input" 
              placeholder="https://dav.your-server.com/" 
              value={webdavUrl}
              onChange={(e) => setWebdavUrl(e.target.value)}
            />
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-3)" }}>
            <div>
              <label style={{ display: "block", marginBottom: "4px", fontSize: "13px" }}>用户名</label>
              <input 
                type="text" 
                className="form-input" 
                value={webdavUser}
                onChange={(e) => setWebdavUser(e.target.value)}
              />
            </div>
            <div>
              <label style={{ display: "block", marginBottom: "4px", fontSize: "13px" }}>密码 / API Token</label>
              <input 
                type="password" 
                className="form-input" 
                value={webdavPass}
                onChange={(e) => setWebdavPass(e.target.value)}
              />
            </div>
          </div>
          <div style={{ display: "flex", gap: "var(--space-3)", marginTop: "var(--space-2)" }}>
            <button className="btn btn-secondary" onClick={handleWebdavTest} disabled={webdavLoading || !webdavUrl}>
              测试连接
            </button>
            <button className="btn btn-primary" onClick={handleWebdavPush} disabled={webdavLoading || !webdavUrl}>
              立即上传推送 (Push)
            </button>
            <button className="btn btn-danger" onClick={handleWebdavPull} disabled={webdavLoading || !webdavUrl}>
              立即下载覆盖 (Pull)
            </button>
          </div>
          {webdavMsg && (
            <div style={{ padding: "var(--space-2)", background: "rgba(0,0,0,0.2)", borderRadius: "var(--radius-sm)", fontSize: 13, marginTop: "var(--space-2)" }}>
              {webdavMsg}
            </div>
          )}
        </div>

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
