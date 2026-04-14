import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

import { check, Update as TauriUpdate } from "@tauri-apps/plugin-updater";
import {
  api,
  type GeminiInstanceRecord,
  type OAuthEnvStatusItem,
  type LinuxReleaseInfo,
  type SkillStorageInfo,
  type UpdateRuntimeInfo,
  type UpdateSettings,
  type WebSocketStatus,
} from "../../lib/api";

export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");
  const [updateMsg, setUpdateMsg] = useState("");
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [updateSettings, setUpdateSettings] = useState<UpdateSettings | null>(null);
  const [updateRuntimeInfo, setUpdateRuntimeInfo] = useState<UpdateRuntimeInfo | null>(null);
  const [linuxReleaseInfo, setLinuxReleaseInfo] = useState<LinuxReleaseInfo | null>(null);
  const [linuxInstallBusyUrl, setLinuxInstallBusyUrl] = useState<string | null>(null);
  const [availableUpdate, setAvailableUpdate] = useState<TauriUpdate | null>(null);
  const [websocketStatus, setWebsocketStatus] = useState<WebSocketStatus | null>(null);
  const [updateProgress, setUpdateProgress] = useState<{ phase: "idle" | "checking" | "downloading" | "installing" | "finished"; downloaded: number; total: number }>({
    phase: "idle",
    downloaded: 0,
    total: 0,
  });

  const [webdavUrl, setWebdavUrl] = useState(localStorage.getItem("webdav_url") || "");
  const [webdavUser, setWebdavUser] = useState(localStorage.getItem("webdav_user") || "");
  const [webdavPass, setWebdavPass] = useState(localStorage.getItem("webdav_pass") || "");
  const [webdavMsg, setWebdavMsg] = useState("");
  const [webdavLoading, setWebdavLoading] = useState(false);
  const [skillStorage, setSkillStorage] = useState<SkillStorageInfo | null>(null);
  const [oauthEnvStatus, setOauthEnvStatus] = useState<OAuthEnvStatusItem[]>([]);
  const [geminiInstances, setGeminiInstances] = useState<GeminiInstanceRecord[]>([]);
  const [defaultGeminiInstance, setDefaultGeminiInstance] = useState<GeminiInstanceRecord | null>(null);
  const [geminiInstanceName, setGeminiInstanceName] = useState("");
  const [geminiInstanceDir, setGeminiInstanceDir] = useState("");
  const [geminiInstanceMsg, setGeminiInstanceMsg] = useState("");
  const [geminiInstanceLoading, setGeminiInstanceLoading] = useState(false);
  const [geminiEditDialog, setGeminiEditDialog] = useState<{
    instance: GeminiInstanceRecord;
    extraArgs: string;
    bindAccountId: string;
    projectId: string;
  } | null>(null);
  const [confirmDeleteGeminiId, setConfirmDeleteGeminiId] = useState<string | null>(null);
  const [confirmWebdavPull, setConfirmWebdavPull] = useState(false);
  const [runtimeLoading, setRuntimeLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;

    const loadRuntimeInfo = async () => {
      setRuntimeLoading(true);
      try {
        const [storageInfo, oauthInfo, instanceList, defaultInstance] = await Promise.all([
          api.skills.getStorageInfo(),
          api.oauth.getEnvStatus(),
          api.geminiInstances.list(),
          api.geminiInstances.getDefault(),
        ]);
        const [runtimeInfo, savedUpdateSettings] = await Promise.all([
          api.update.getRuntimeInfo(),
          api.update.getSettings(),
        ]);
        const wsStatus = await api.websocket.getStatus();
        if (!cancelled) {
          setSkillStorage(storageInfo);
          setOauthEnvStatus(oauthInfo);
          setGeminiInstances(instanceList);
          setDefaultGeminiInstance(defaultInstance);
          setUpdateRuntimeInfo(runtimeInfo);
          setUpdateSettings(savedUpdateSettings);
          setWebsocketStatus(wsStatus);
        }
        if (!cancelled && runtimeInfo.platform === "linux") {
          api.update.getLinuxReleaseInfo().then(setLinuxReleaseInfo).catch((error) => {
            console.warn("Failed to load Linux release info:", error);
          });
        }
      } catch (e) {
        if (!cancelled) {
          console.error("Failed to load runtime info:", e);
        }
      } finally {
        if (!cancelled) setRuntimeLoading(false);
      }
    };

    loadRuntimeInfo();
    return () => {
      cancelled = true;
    };
  }, []);

  const reloadGeminiInstances = async () => {
    const [instanceList, defaultInstance] = await Promise.all([
      api.geminiInstances.list(),
      api.geminiInstances.getDefault(),
    ]);
    setGeminiInstances(instanceList);
    setDefaultGeminiInstance(defaultInstance);
  };

  const handlePickGeminiDir = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "选择 Gemini 实例目录",
    });
    if (typeof selected === "string") {
      setGeminiInstanceDir(selected);
    }
  };

  const handleAddGeminiInstance = async () => {
    if (!geminiInstanceName.trim() || !geminiInstanceDir.trim()) {
      setGeminiInstanceMsg("请填写实例名称并选择目录");
      return;
    }
    setGeminiInstanceLoading(true);
    try {
      await api.geminiInstances.add(geminiInstanceName.trim(), geminiInstanceDir.trim());
      setGeminiInstanceName("");
      setGeminiInstanceDir("");
      setGeminiInstanceMsg("Gemini 实例已添加");
      await reloadGeminiInstances();
    } catch (e) {
      setGeminiInstanceMsg(`添加 Gemini 实例失败: ${e}`);
    } finally {
      setGeminiInstanceLoading(false);
    }
  };

  const handleUpdateGeminiInstance = async (instance: GeminiInstanceRecord) => {
    setGeminiEditDialog({
      instance,
      extraArgs: instance.extra_args || "",
      bindAccountId: instance.bind_account_id || "",
      projectId: instance.project_id || "",
    });
  };

  const handleCopyGeminiLaunchCommand = async (id: string) => {
    try {
      const info = await api.geminiInstances.getLaunchCommand(id);
      await navigator.clipboard.writeText(info.launch_command);
      setGeminiInstanceMsg("Gemini 启动命令已复制到剪贴板");
    } catch (e) {
      setGeminiInstanceMsg(`读取 Gemini 启动命令失败: ${e}`);
    }
  };

  const handleLaunchGeminiInstance = async (id: string) => {
    try {
      const message = await api.geminiInstances.launch(id);
      setGeminiInstanceMsg(message);
      await reloadGeminiInstances();
    } catch (e) {
      setGeminiInstanceMsg(`启动 Gemini 实例失败: ${e}`);
    }
  };

  const handleDeleteGeminiInstance = async (id: string) => {
    try {
      await api.geminiInstances.delete(id);
      setGeminiInstanceMsg("Gemini 实例已删除");
      setConfirmDeleteGeminiId(null);
      await reloadGeminiInstances();
    } catch (e) {
      setGeminiInstanceMsg(`删除 Gemini 实例失败: ${e}`);
    }
  };

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
    try {
      setWebdavLoading(true);
      setWebdavMsg("正在从云端拉取配置...");
      await api.webdav.pull(saveWebdavConfig());
      setWebdavMsg("✅ 拉取成功！数据已应用。");
      setConfirmWebdavPull(false);
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
    setUpdateProgress({ phase: "checking", downloaded: 0, total: 0 });
    setUpdateMsg("正在请求更新服务器...");
    try {
      const settings = await api.update.markCheckedNow();
      setUpdateSettings(settings);
      const update = await check();
      if (update) {
        setAvailableUpdate(update);
        setUpdateMsg(`发现新版本 ${update.version}`);
        if (updateSettings?.auto_install && updateRuntimeInfo?.can_auto_install !== false) {
          await handleInstallUpdate(update);
        } else {
          setUpdateProgress({ phase: "idle", downloaded: 0, total: 0 });
        }
      } else {
        setAvailableUpdate(null);
        setUpdateMsg("当前已是最新版本");
        setUpdateProgress({ phase: "finished", downloaded: 0, total: 0 });
      }
    } catch (e) {
      setUpdateMsg(`检查更新失败: ${String(e)} (可能是因为测试地址或网络问题)`);
      setUpdateProgress({ phase: "idle", downloaded: 0, total: 0 });
    } finally {
      setIsCheckingUpdate(false);
    }
  };

  const handleInstallUpdate = async (update = availableUpdate) => {
    if (!update) return;
    try {
      let downloaded = 0;
      let total = 0;
      setUpdateProgress({ phase: "downloading", downloaded: 0, total: 0 });
      setUpdateMsg(`正在下载 ${update.version}...`);
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength || 0;
            setUpdateProgress({ phase: "downloading", downloaded: 0, total });
            setUpdateMsg(total > 0 ? `开始下载 ${update.version}（共 ${total} bytes）` : `开始下载 ${update.version}`);
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            setUpdateProgress({ phase: "downloading", downloaded, total });
            setUpdateMsg(total > 0 ? `已下载 ${downloaded} / ${total}` : `已下载 ${downloaded} bytes`);
            break;
          case "Finished":
            setUpdateProgress({ phase: "installing", downloaded, total });
            setUpdateMsg("下载完成，正在安装更新...");
            break;
        }
      });
      setUpdateProgress({ phase: "finished", downloaded, total });
      setUpdateMsg("更新已安装！需要重启应用后生效。");
    } catch (e) {
      setUpdateProgress({ phase: "idle", downloaded: 0, total: 0 });
      setUpdateMsg(`安装更新失败: ${String(e)}`);
    }
  };

  const handleUpdateSettingChange = async (
    patch: Partial<UpdateSettings>,
    successMessage?: string,
  ) => {
    if (!updateSettings) return;
    const next = { ...updateSettings, ...patch };
    setUpdateSettings(next);
    try {
      await api.update.saveSettings(next);
      if (successMessage) setUpdateMsg(successMessage);
    } catch (e) {
      setUpdateMsg(`保存更新设置失败: ${e}`);
      setUpdateSettings(updateSettings);
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

        <h3 style={{ marginBottom: "var(--space-2)" }}>运行时状态</h3>
        <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
          把当前应用真正依赖的本地路径和 OAuth 环境配置显式展示出来，方便排查问题。
        </p>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-4)", marginBottom: "var(--space-6)" }}>
          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)" }}>
            <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>Skills 存储位置</div>
            {runtimeLoading ? (
              <div className="text-muted" style={{ fontSize: 13 }}>加载中...</div>
            ) : skillStorage ? (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>主仓路径</div>
                  <code style={{ fontSize: 12, wordBreak: "break-all" }}>{skillStorage.primary_path}</code>
                </div>
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>兼容旧目录</div>
                  <code style={{ fontSize: 12, wordBreak: "break-all" }}>{skillStorage.legacy_path}</code>
                </div>
                <div style={{ fontSize: 12, color: skillStorage.legacy_exists ? "var(--color-warning)" : "var(--color-success)" }}>
                  {skillStorage.legacy_exists ? "检测到旧目录，应用会继续兼容读取。" : "未检测到旧目录，当前已完全使用新技能仓。"}
                </div>
              </div>
            ) : (
              <div className="text-muted" style={{ fontSize: 13 }}>未能读取技能仓信息</div>
            )}
          </div>

          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)" }}>
            <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>OAuth 环境配置</div>
            {runtimeLoading ? (
              <div className="text-muted" style={{ fontSize: 13 }}>加载中...</div>
            ) : oauthEnvStatus.length > 0 ? (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
                {oauthEnvStatus.map((item) => (
                  <div key={item.env_name} style={{ paddingBottom: "var(--space-2)", borderBottom: "1px solid var(--color-border)" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                      <span style={{ fontWeight: 500, fontSize: 13 }}>{item.provider}</span>
                      <span style={{ fontSize: 12, color: item.configured ? "var(--color-success)" : "var(--color-danger)" }}>
                        {item.configured ? "已配置" : "缺失"}
                      </span>
                    </div>
                    <code style={{ fontSize: 12, wordBreak: "break-all" }}>{item.env_name}</code>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-muted" style={{ fontSize: 13 }}>当前没有需要展示的 OAuth 环境项</div>
            )}
          </div>

          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)" }}>
            <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>本地 WebSocket 广播</div>
            {runtimeLoading ? (
              <div className="text-muted" style={{ fontSize: 13 }}>加载中...</div>
            ) : websocketStatus ? (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
                <div style={{ fontSize: 13 }}>
                  状态：
                  <strong style={{ marginLeft: 6 }}>{websocketStatus.running ? "运行中" : "未启动"}</strong>
                </div>
                <div style={{ fontSize: 13 }}>
                  端口：
                  <code style={{ marginLeft: 6 }}>{websocketStatus.port ?? "—"}</code>
                </div>
                <div style={{ fontSize: 13 }}>
                  客户端数：
                  <strong style={{ marginLeft: 6 }}>{websocketStatus.client_count}</strong>
                </div>
              </div>
            ) : (
              <div className="text-muted" style={{ fontSize: 13 }}>未能读取 WebSocket 状态</div>
            )}
          </div>
        </div>

        <h3 style={{ marginBottom: "var(--space-2)" }}>Gemini 实例</h3>
        <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
          管理 Gemini CLI 的默认实例与额外实例目录，支持实例级绑定账号、项目 ID 和启动参数。
        </p>
        <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)", marginBottom: "var(--space-6)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
          {defaultGeminiInstance && (
            <div style={{ padding: "var(--space-3)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", background: "rgba(255,255,255,0.02)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", gap: "var(--space-3)" }}>
                <div>
                  <div style={{ fontWeight: 600 }}>默认实例</div>
                  <div className="text-muted" style={{ fontSize: 12, wordBreak: "break-all" }}>{defaultGeminiInstance.user_data_dir}</div>
                  <div className="text-muted" style={{ fontSize: 12, marginTop: 6 }}>
                    {defaultGeminiInstance.bind_account_id ? `绑定账号 ${defaultGeminiInstance.bind_account_id}` : "未绑定账号"}
                    {" · "}
                    {defaultGeminiInstance.project_id ? `项目 ${defaultGeminiInstance.project_id}` : "无项目覆盖"}
                    {" · "}
                    {defaultGeminiInstance.extra_args ? `参数 ${defaultGeminiInstance.extra_args}` : "无额外参数"}
                  </div>
                </div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap", justifyContent: "flex-end" }}>
                  <button className="btn btn-secondary" onClick={() => handleUpdateGeminiInstance(defaultGeminiInstance)}>设置</button>
                  <button className="btn btn-secondary" onClick={() => handleCopyGeminiLaunchCommand(defaultGeminiInstance.id)}>复制命令</button>
                  <button className="btn btn-primary" onClick={() => handleLaunchGeminiInstance(defaultGeminiInstance.id)}>在终端启动</button>
                </div>
              </div>
            </div>
          )}

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr auto", gap: "var(--space-3)", alignItems: "end" }}>
            <div>
              <label style={{ display: "block", marginBottom: 4, fontSize: 13 }}>实例名称</label>
              <input
                type="text"
                className="form-input"
                value={geminiInstanceName}
                onChange={(e) => setGeminiInstanceName(e.target.value)}
                placeholder="例如：工作区实例 / 沙盒实例"
              />
            </div>
            <div>
              <label style={{ display: "block", marginBottom: 4, fontSize: 13 }}>实例目录</label>
              <input
                type="text"
                className="form-input"
                value={geminiInstanceDir}
                onChange={(e) => setGeminiInstanceDir(e.target.value)}
                placeholder="选择或粘贴实例目录"
              />
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button className="btn btn-secondary" onClick={handlePickGeminiDir}>浏览</button>
              <button className="btn btn-primary" onClick={handleAddGeminiInstance} disabled={geminiInstanceLoading}>
                {geminiInstanceLoading ? "添加中..." : "添加实例"}
              </button>
            </div>
          </div>

          {geminiInstanceMsg && (
            <div style={{ padding: "var(--space-2)", background: "rgba(0,0,0,0.2)", borderRadius: "var(--radius-sm)", fontSize: 13 }}>
              {geminiInstanceMsg}
            </div>
          )}

          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
            {geminiInstances.length === 0 ? (
              <div className="text-muted" style={{ fontSize: 13 }}>当前还没有额外 Gemini 实例</div>
            ) : (
              geminiInstances.map((instance) => (
                <div key={instance.id} style={{ padding: "var(--space-3)", border: "1px solid var(--color-border)", borderRadius: "var(--radius-sm)", background: "rgba(255,255,255,0.02)" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", gap: "var(--space-3)" }}>
                    <div>
                      <div style={{ fontWeight: 600 }}>{instance.name}</div>
                      <div className="text-muted" style={{ fontSize: 12, wordBreak: "break-all" }}>{instance.user_data_dir}</div>
                      <div className="text-muted" style={{ fontSize: 12, marginTop: 6 }}>
                        {instance.bind_account_id ? `绑定账号 ${instance.bind_account_id}` : "未绑定账号"}
                        {" · "}
                        {instance.project_id ? `项目 ${instance.project_id}` : "无项目覆盖"}
                        {" · "}
                        {instance.extra_args ? `参数 ${instance.extra_args}` : "无额外参数"}
                        {" · "}
                        {instance.initialized ? "已初始化" : "未初始化"}
                      </div>
                    </div>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap", justifyContent: "flex-end" }}>
                      <button className="btn btn-secondary" onClick={() => handleUpdateGeminiInstance(instance)}>设置</button>
                      <button className="btn btn-secondary" onClick={() => handleCopyGeminiLaunchCommand(instance.id)}>复制命令</button>
                      <button className="btn btn-primary" onClick={() => handleLaunchGeminiInstance(instance.id)}>在终端启动</button>
                      <button className="btn btn-danger" onClick={() => setConfirmDeleteGeminiId(instance.id)}>删除</button>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
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
            <button className="btn btn-danger" onClick={() => setConfirmWebdavPull(true)} disabled={webdavLoading || !webdavUrl}>
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
        <div style={{ marginBottom: "var(--space-6)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)", display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "var(--space-3)" }}>
            <div>
              <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>当前版本</div>
              <div style={{ fontWeight: 600 }}>{updateRuntimeInfo?.current_version || "—"}</div>
            </div>
            <div>
              <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>平台</div>
              <div style={{ fontWeight: 600 }}>
                {updateRuntimeInfo?.platform || "—"}
                {updateRuntimeInfo?.linux_install_kind ? ` · ${updateRuntimeInfo.linux_install_kind}` : ""}
              </div>
            </div>
            <div>
              <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>上次检查</div>
              <div style={{ fontWeight: 600 }}>
                {updateSettings?.last_check_at ? new Date(updateSettings.last_check_at).toLocaleString() : "尚未检查"}
              </div>
            </div>
          </div>

          <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
            <label style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 13 }}>
              <input
                type="checkbox"
                checked={!!updateSettings?.auto_check}
                onChange={(e) => handleUpdateSettingChange({ auto_check: e.target.checked }, "更新设置已保存")}
              />
              自动检查更新
            </label>
            <label style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 13, opacity: updateRuntimeInfo?.can_auto_install === false ? 0.65 : 1 }}>
              <input
                type="checkbox"
                checked={!!updateSettings?.auto_install}
                disabled={updateRuntimeInfo?.can_auto_install === false}
                onChange={(e) => handleUpdateSettingChange({ auto_install: e.target.checked }, "更新设置已保存")}
              />
              自动安装更新
            </label>
            {updateRuntimeInfo?.warning && (
              <div className="alert alert-info" style={{ fontSize: 13 }}>
                {updateRuntimeInfo.warning}
              </div>
            )}
            {updateRuntimeInfo?.linux_manual_hint && (
              <div style={{ padding: "var(--space-3)", background: "rgba(0,0,0,0.18)", borderRadius: "var(--radius-sm)", fontSize: 13, lineHeight: 1.6 }}>
                <div style={{ fontWeight: 600, marginBottom: 6 }}>Linux 安装处理建议</div>
                <div>{updateRuntimeInfo.linux_manual_hint}</div>
              </div>
            )}
            {updateRuntimeInfo?.platform === "linux" && linuxReleaseInfo && linuxReleaseInfo.assets.length > 0 && (
              <div style={{ padding: "var(--space-3)", background: "rgba(0,0,0,0.18)", borderRadius: "var(--radius-sm)", display: "flex", flexDirection: "column", gap: 10 }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
                  <div>
                    <div style={{ fontWeight: 600 }}>Linux 发行包资产</div>
                    <div className="text-muted" style={{ fontSize: 12 }}>
                      {linuxReleaseInfo.version}
                      {linuxReleaseInfo.published_at ? ` · ${new Date(linuxReleaseInfo.published_at).toLocaleString()}` : ""}
                    </div>
                  </div>
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  {linuxReleaseInfo.assets.map((asset) => (
                    <div key={asset.url} style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap", padding: "10px 12px", borderRadius: "var(--radius-sm)", border: "1px solid var(--color-border)" }}>
                      <div>
                        <div style={{ fontWeight: 500 }}>
                          {asset.name}
                          {asset.preferred ? " · 推荐" : ""}
                        </div>
                        <div className="text-muted" style={{ fontSize: 12 }}>
                          {asset.kind}
                          {typeof asset.size === "number" ? ` · ${Math.round(asset.size / 1024)} KB` : ""}
                        </div>
                      </div>
                      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                        <button className="btn btn-secondary" onClick={() => api.update.openAssetUrl(asset.url)}>
                          下载此安装包
                        </button>
                        <button
                          className="btn btn-primary"
                          disabled={linuxInstallBusyUrl === asset.url}
                          onClick={async () => {
                            try {
                              setLinuxInstallBusyUrl(asset.url);
                              const result = await api.update.installLinuxAsset({
                                url: asset.url,
                                kind: asset.kind,
                                version: linuxReleaseInfo.version,
                              });
                              setUpdateMsg(`${result.message} 路径：${result.downloaded_path}`);
                            } catch (e) {
                              setUpdateMsg(`Linux 安装执行失败: ${e}`);
                            } finally {
                              setLinuxInstallBusyUrl(null);
                            }
                          }}
                        >
                          {linuxInstallBusyUrl === asset.url ? "处理中..." : (asset.kind === "appimage" ? "下载并准备" : "下载并安装")}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
            <div>
              <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>Updater Endpoints</div>
              {updateRuntimeInfo?.updater_endpoints?.length ? (
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {updateRuntimeInfo.updater_endpoints.map((endpoint) => (
                    <code key={endpoint} style={{ fontSize: 12, wordBreak: "break-all" }}>{endpoint}</code>
                  ))}
                </div>
              ) : (
                <div className="text-muted" style={{ fontSize: 13 }}>当前未读取到更新地址</div>
              )}
            </div>
          </div>

          <div style={{ display: "flex", gap: "var(--space-3)", alignItems: "center", flexWrap: "wrap" }}>
          <button 
            className="btn btn-primary"
            onClick={handleCheckUpdate}
            disabled={isCheckingUpdate}
          >
            {isCheckingUpdate ? "检查中..." : t("settings.check_now")}
          </button>
          <span className="text-muted" style={{ fontSize: 12 }}>
            {updateRuntimeInfo?.updater_pubkey_configured ? "Updater 公钥已配置" : "Updater 公钥仍为占位值"}
          </span>
          </div>
          {availableUpdate && (
            <div style={{ background: "var(--surface-sunken)", padding: "var(--space-4)", borderRadius: "var(--radius-md)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "var(--space-3)" }}>
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>发现版本</div>
                  <div style={{ fontWeight: 600 }}>{availableUpdate.version}</div>
                </div>
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>发布日期</div>
                  <div style={{ fontWeight: 600 }}>
                    {availableUpdate.date ? new Date(availableUpdate.date).toLocaleString() : "未知"}
                  </div>
                </div>
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>安装能力</div>
                  <div style={{ fontWeight: 600 }}>
                    {updateRuntimeInfo?.can_auto_install ? "支持插件自动安装" : "建议手动处理"}
                  </div>
                </div>
              </div>

              {availableUpdate.body && (
                <div>
                  <div className="text-muted" style={{ fontSize: 12, marginBottom: 6 }}>发布说明</div>
                  <pre style={{ margin: 0, whiteSpace: "pre-wrap", fontSize: 12, lineHeight: 1.5, padding: "var(--space-3)", background: "rgba(0,0,0,0.18)", borderRadius: "var(--radius-sm)" }}>
                    {availableUpdate.body}
                  </pre>
                </div>
              )}

              {updateProgress.phase !== "idle" && (
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12 }}>
                    <span>
                      {updateProgress.phase === "checking" && "正在检查更新"}
                      {updateProgress.phase === "downloading" && "正在下载更新包"}
                      {updateProgress.phase === "installing" && "正在安装更新"}
                      {updateProgress.phase === "finished" && "更新流程完成"}
                    </span>
                    <span>
                      {updateProgress.total > 0 ? `${updateProgress.downloaded} / ${updateProgress.total}` : updateProgress.downloaded > 0 ? `${updateProgress.downloaded} bytes` : ""}
                    </span>
                  </div>
                  <div style={{ height: 8, borderRadius: 999, background: "rgba(255,255,255,0.08)", overflow: "hidden" }}>
                    <div
                      style={{
                        height: "100%",
                        width: updateProgress.total > 0 ? `${Math.min(100, (updateProgress.downloaded / updateProgress.total) * 100)}%` : updateProgress.phase === "finished" ? "100%" : "20%",
                        background: "linear-gradient(90deg, var(--color-primary), var(--color-accent))",
                        transition: "width 180ms ease",
                      }}
                    />
                  </div>
                </div>
              )}

              <div style={{ display: "flex", gap: "var(--space-3)", flexWrap: "wrap" }}>
                <button
                  className="btn btn-primary"
                  disabled={isCheckingUpdate || updateProgress.phase === "downloading" || updateProgress.phase === "installing" || updateRuntimeInfo?.can_auto_install === false}
                  onClick={() => handleInstallUpdate()}
                >
                  {updateProgress.phase === "downloading" || updateProgress.phase === "installing" ? "处理中..." : "下载并安装"}
                </button>
                <button
                  className="btn btn-secondary"
                  disabled={isCheckingUpdate}
                  onClick={() => setAvailableUpdate(null)}
                >
                  收起更新详情
                </button>
              </div>
            </div>
          )}
          {updateMsg && (
            <div className="alert alert-info" style={{ fontSize: 13, alignSelf: "stretch" }}>
              {updateMsg}
            </div>
          )}
        </div>

      </div>

      {geminiEditDialog && (
        <div className="modal-overlay" onClick={() => setGeminiEditDialog(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>设置 Gemini 实例</h2>
              <button className="btn btn-icon" onClick={() => setGeminiEditDialog(null)}>✕</button>
            </div>
            <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <div>
                <label className="form-label">额外启动参数</label>
                <input className="form-input" value={geminiEditDialog.extraArgs} onChange={(e) => setGeminiEditDialog({ ...geminiEditDialog, extraArgs: e.target.value })} />
              </div>
              <div>
                <label className="form-label">绑定账号 ID</label>
                <input className="form-input" value={geminiEditDialog.bindAccountId} onChange={(e) => setGeminiEditDialog({ ...geminiEditDialog, bindAccountId: e.target.value })} />
              </div>
              <div>
                <label className="form-label">项目 ID</label>
                <input className="form-input" value={geminiEditDialog.projectId} onChange={(e) => setGeminiEditDialog({ ...geminiEditDialog, projectId: e.target.value })} />
              </div>
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={() => setGeminiEditDialog(null)}>取消</button>
                <button
                  className="btn btn-primary"
                  onClick={async () => {
                    try {
                      await api.geminiInstances.update(
                        geminiEditDialog.instance.id,
                        geminiEditDialog.extraArgs,
                        geminiEditDialog.bindAccountId.trim() || null,
                        geminiEditDialog.projectId.trim() || null,
                      );
                      setGeminiInstanceMsg("Gemini 实例设置已更新");
                      setGeminiEditDialog(null);
                      await reloadGeminiInstances();
                    } catch (e) {
                      setGeminiInstanceMsg(`更新 Gemini 实例失败: ${e}`);
                    }
                  }}
                >
                  保存
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {confirmDeleteGeminiId && (
        <div className="modal-overlay" onClick={() => setConfirmDeleteGeminiId(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>删除 Gemini 实例</h2>
              <button className="btn btn-icon" onClick={() => setConfirmDeleteGeminiId(null)}>✕</button>
            </div>
            <div className="modal-body">
              <p>确认删除这个 Gemini 实例目录索引吗？不会删除真实文件。</p>
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={() => setConfirmDeleteGeminiId(null)}>取消</button>
                <button className="btn btn-danger" onClick={() => handleDeleteGeminiInstance(confirmDeleteGeminiId)}>删除</button>
              </div>
            </div>
          </div>
        </div>
      )}

      {confirmWebdavPull && (
        <div className="modal-overlay" onClick={() => !webdavLoading && setConfirmWebdavPull(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>确认 WebDAV Pull</h2>
              <button className="btn btn-icon" onClick={() => setConfirmWebdavPull(false)}>✕</button>
            </div>
            <div className="modal-body">
              <p>警告：拉取将会用云端配置覆盖本地配置（增量覆盖），确定要继续吗？</p>
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={() => setConfirmWebdavPull(false)} disabled={webdavLoading}>取消</button>
                <button className="btn btn-danger" onClick={handleWebdavPull} disabled={webdavLoading}>
                  {webdavLoading ? "拉取中..." : "确认拉取"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
